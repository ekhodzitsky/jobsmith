//! Prompt builders for the Kimi Code AI workflow.
//!
//! Each function builds a structured prompt string for a specific stage
//! in the drafter-reviewer pipeline.

use crate::hh::models::VacancyDetail;
use crate::profile::model::Profile;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FIT_EVAL_DESCRIPTION_LIMIT: usize = 3000;
const PROMPT_DESCRIPTION_LIMIT: usize = 2000;
const COVER_DRAFT_DESCRIPTION_LIMIT: usize = 1500;
const COMPACT_PROFILE_SUMMARY_LIMIT: usize = 500;
const RECENT_EXPERIENCE_COUNT: usize = 2;
const TOP_SKILLS_COUNT: usize = 10;
const TRUNCATE_SUFFIX_MARGIN: usize = 15;

/// Build the fit evaluation prompt.
///
/// Asks Kimi to evaluate how well the candidate fits the vacancy.
pub fn build_fit_evaluation_prompt(profile: &Profile, vacancy: &VacancyDetail) -> String {
    let vacancy_desc = vacancy
        .base
        .description
        .as_deref()
        .map(crate::hh::models::strip_html)
        .unwrap_or_default();

    let skills = vacancy
        .base
        .key_skills
        .as_ref()
        .map(|s| s.iter().filter_map(|k| k.name.clone()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();

    let salary = vacancy
        .base
        .salary
        .as_ref()
        .map(|s| {
            let from = s.from.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
            let to = s.to.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
            let currency = s.currency.as_deref().unwrap_or("RUR");
            format!("{from} - {to} {currency}")
        })
        .unwrap_or_else(|| "не указана".to_string());

    let experience = vacancy
        .base
        .experience
        .as_ref()
        .and_then(|e| e.name.clone())
        .unwrap_or_else(|| "не указан".to_string());

    let schedule = vacancy
        .base
        .schedule
        .as_ref()
        .and_then(|s| s.name.clone())
        .unwrap_or_else(|| "не указан".to_string());

    format!(
        r#"Ты — карьерный консультант. Оцени совместимость кандидата с вакансией.

## Профиль кандидата

**Имя:** {name}
**Город:** {city}
**Гражданство:** {citizenship}
**Военная обязанность:** {military}
**Опыт работы:** {exp_years} лет
**Навыки:** {candidate_skills}
**Целевая роль:** {target_roles}
**Целевая зарплата:** {target_salary}
**Формат работы:** {work_format}
**Готовность к релокации:** {relocate}

**Опыт:**
{experience_text}

**Образование:**
{education_text}

## Вакансия

**Компания:** {company}
**Роль:** {role}
**Зарплата:** {salary}
**Опыт:** {experience_req}
**График:** {schedule}
**Место:** {location}

**Описание:**
{description}

**Ключевые навыки:** {skills}

## Инструкция

Оцени совместимость по шкале 0-100. Учитывай:
1. Совпадение hard skills
2. Релевантность опыта
3. Соответствие зарплатным ожиданиям
4. Локация / формат работы
5. Культурная совместимость (по описанию)

Ответь в формате:

SCORE: <число 0-100>
VERDICT: <accept / reject / marginal>
REASONING: <2-3 абзаца обоснования>
GAPS: <что не хватает кандидату>
STRENGTHS: <что выделяет кандидата>
"#,
        name = profile.name,
        city = profile.city,
        citizenship = profile.citizenship,
        military = profile
            .military_status
            .as_ref()
            .map(|m| format!("{m:?}"))
            .unwrap_or_else(|| "не указан".to_string()),
        exp_years = profile.experience.len(),
        candidate_skills = profile.skills.join(", "),
        target_roles = profile.target_roles.join(", "),
        target_salary = profile
            .target_salary
            .map(|s| format!("{s} {}", profile.target_currency.as_deref().unwrap_or("RUR")))
            .unwrap_or_else(|| "не указана".to_string()),
        work_format = profile.work_format.as_deref().unwrap_or("не указан"),
        relocate = if profile.ready_to_relocate { "да" } else { "нет" },
        experience_text = format_experience(profile),
        education_text = format_education(profile),
        company = vacancy.base.employer_name(),
        role = vacancy.base.name,
        salary = salary,
        experience_req = experience,
        schedule = schedule,
        location = vacancy
            .base
            .area
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "не указан".to_string()),
        description = truncate(&vacancy_desc, FIT_EVAL_DESCRIPTION_LIMIT),
        skills = skills,
    )
}

/// Build the CV drafting prompt.
pub fn build_cv_draft_prompt(
    profile: &Profile,
    vacancy: &VacancyDetail,
    evaluation: &str,
) -> String {
    format!(
        r#"Ты — профессиональный рекрутер и карьерный консультант. Составь резюме кандидата для конкретной вакансии.

## Контекст

{evaluation}

## Профиль кандидата

{profile_summary}

## Вакансия

**Компания:** {company}
**Роль:** {role}
**Описание:** {description}

## Инструкция

Составь резюме в формате markdown со следующими секциями:

1. **Контактная информация** — имя, телефон, email, город, ссылки
2. **Профессиональная позиция** — 1-2 предложения, заточенные под роль
3. **Навыки** — релевантные вакансии (не все подряд)
4. **Опыт работы** — адаптирован под требования вакансии, с measurable achievements
5. **Образование** — релевантное
6. **Сертификаты** — релевантные
7. **Языки**
8. **Дополнительно** — публикации, open source, если релевантно

Правила:
- Все факты должны соответствовать профилю (не выдумывай)
- Акцентируй внимание на том, что совпадает с вакансией
- Используй action verbs и количественные результаты
- Резюме должно быть на 1-2 страницы (компактное)
- Пиши на русском языке
- Упоминай Kimi Code если релевантно (агентное программирование)

Ответь только markdown-резюме, без обрамления.
"#,
        evaluation = evaluation,
        profile_summary = build_compact_profile_summary(profile),
        company = vacancy.base.employer_name(),
        role = vacancy.base.name,
        description = truncate(
            &vacancy
                .base
                .description
                .as_deref()
                .map(crate::hh::models::strip_html)
                .unwrap_or_default(),
            PROMPT_DESCRIPTION_LIMIT
        ),
    )
}

/// Build the cover letter drafting prompt.
pub fn build_cover_draft_prompt(
    profile: &Profile,
    vacancy: &VacancyDetail,
    cv_draft: &str,
) -> String {
    format!(
        r#"Ты — профессиональный карьерный консультант. Напиши сопроводительное письмо.

## Профиль кандидата

{profile_summary}

## Вакансия

**Компания:** {company}
**Роль:** {role}
**Описание:** {description}

## Резюме (черновик)

{cv_draft}

## Инструкция

Напиши сопроводительное письмо:

1. **Приветствие** — "Уважаемый менеджер по подбору персонала" или конкретное имя если известно
2. **Вступление** — почему эта роль и компания
3. **Тело** — 2-3 параграфа о релевантном опыте и навыках
4. **Заключение** — призыв к действию (собеседование)
5. **Подпись** — с контактами

Правила:
- Письмо должно быть на 1 страницу (200-300 слов)
- Не повторяй дословно резюме — дополняй его
- Покажи заинтересованность в конкретной компании (упомяни продукт/направление)
- Пиши на русском языке
- Тон: уважительный, уверенный, не навязчивый

Ответь только текст письма, без обрамления.
"#,
        profile_summary = format_profile_summary(profile),
        company = vacancy.base.employer_name(),
        role = vacancy.base.name,
        description = truncate(
            &vacancy
                .base
                .description
                .as_deref()
                .map(crate::hh::models::strip_html)
                .unwrap_or_default(),
            COVER_DRAFT_DESCRIPTION_LIMIT
        ),
        cv_draft = cv_draft,
    )
}

/// Build the reviewer prompt.
pub fn build_reviewer_prompt(
    _profile: &Profile,
    vacancy: &VacancyDetail,
    cv_draft: &str,
    cover_draft: &str,
) -> String {
    format!(
        r#"Ты — опытный технический рекрутер. Проанализируй резюме и сопроводительное письмо кандидата.

## Вакансия

**Компания:** {company}
**Роль:** {role}
**Описание:** {description}

## Резюме кандидата

{cv_draft}

## Сопроводительное письмо

{cover_draft}

## Инструкция

Проведи критический анализ. Оцени по шкале 1-5:

1. **Релевантность** — насколько документы соответствуют вакансии
2. **Фактическая точность** — все ли утверждения проверяемы
3. **Таргетированность** — адаптированы ли документы под конкретную роль
4. **Язык и стиль** — грамотность, тон, структура
5. **Уникальность** — есть ли шаблонные фразы

Дай конкретные рекомендации по улучшению:
- Что добавить
- Что убрать
- Что переформулировать
- Какие soft skills подчеркнуть
- Какие gap'ы адресовать

Формат:

SCORES:
- relevance: <1-5>
- accuracy: <1-5>
- targeting: <1-5>
- language: <1-5>
- uniqueness: <1-5>

CRITIQUE:
<детальный разбор>

ACTION_ITEMS:
<список конкретных правок>
"#,
        company = vacancy.base.employer_name(),
        role = vacancy.base.name,
        description = truncate(
            &vacancy
                .base
                .description
                .as_deref()
                .map(crate::hh::models::strip_html)
                .unwrap_or_default(),
            PROMPT_DESCRIPTION_LIMIT
        ),
        cv_draft = cv_draft,
        cover_draft = cover_draft,
    )
}

/// Build the revision prompt.
pub fn build_revision_prompt(
    profile: &Profile,
    cv_draft: &str,
    cover_draft: &str,
    review: &str,
) -> String {
    format!(
        r#"Ты — карьерный консультант. Исправь резюме и сопроводительное письмо на основе ревью.

## Профиль кандидата (источник правды)

{profile_summary}

## Черновик резюме

{cv_draft}

## Черновик сопроводительного письма

{cover_draft}

## Ревью

{review}

## Инструкция

Внеси правки согласно ревью. Не выдумывай факты — проверяй каждое утверждение по профилю.

Верни два документа, разделённые маркером:

---CV---
<markdown-резюме>

---COVER---
<текст письма>
"#,
        profile_summary = build_compact_profile_summary(profile),
        cv_draft = cv_draft,
        cover_draft = cover_draft,
        review = review,
    )
}

/// Build the interview prep prompt.
pub fn build_interview_prep_prompt(profile: &Profile, vacancy: &VacancyDetail) -> String {
    format!(
        r#"Ты — карьерный коуч. Подготовь кандидата к собеседованию.

## Профиль

{profile_summary}

## Вакансия

**Компания:** {company}
**Роль:** {role}
**Описание:** {description}

## Инструкция

Подготовь:

1. **Предполагаемые вопросы** (10-15 штук) по роли и опыту
2. **STAR-ответы** на 5 ключевых вопросов
3. **Вопросы кандидату компании** (5-7 штук)
4. **Слабые стороны** — как адресовать gap'ы
5. **Ключевые talking points** — что подчеркнуть

Формат: markdown, на русском.
"#,
        profile_summary = format_profile_summary(profile),
        company = vacancy.base.employer_name(),
        role = vacancy.base.name,
        description = truncate(
            &vacancy
                .base
                .description
                .as_deref()
                .map(crate::hh::models::strip_html)
                .unwrap_or_default(),
            PROMPT_DESCRIPTION_LIMIT
        ),
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_experience(profile: &Profile) -> String {
    profile
        .experience
        .iter()
        .map(|e| {
            let end = e
                .end_date
                .map(|d| d.to_string())
                .unwrap_or_else(|| "настоящее время".to_string());
            format!(
                "- **{role}** @ {company} ({start} - {end})\n  {responsibilities}",
                role = e.role,
                company = e.company,
                start = e.start_date,
                end = end,
                responsibilities = e.responsibilities.join("; "),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_education(profile: &Profile) -> String {
    profile
        .education
        .iter()
        .map(|e| {
            let end = e
                .end_date
                .map(|d| d.to_string())
                .unwrap_or_else(|| "настоящее время".to_string());
            format!(
                "- **{degree}** в {field}, {institution} ({start} - {end})",
                degree = e.degree,
                field = e.field,
                institution = e.institution,
                start = e.start_date,
                end = end,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_profile_summary(profile: &Profile) -> String {
    format!(
        "**{name}**, {city}\n\n**Опыт:** {exp} лет\n**Навыки:** {skills}\n**Образование:** {edu}",
        name = profile.name,
        city = profile.city,
        exp = profile.experience.len(),
        skills = profile.skills.join(", "),
        edu = profile
            .education
            .first()
            .map(|e| format!("{} в {}", e.degree, e.institution))
            .unwrap_or_else(|| "не указано".to_string()),
    )
}

fn build_compact_profile_summary(profile: &Profile) -> String {
    let experience_summary = profile
        .experience
        .iter()
        .rev()
        .take(RECENT_EXPERIENCE_COUNT)
        .map(|e| {
            let end = e
                .end_date
                .map(|d| d.to_string())
                .unwrap_or_else(|| "настоящее время".to_string());
            format!("{} @ {} ({} - {})", e.role, e.company, e.start_date, end)
        })
        .collect::<Vec<_>>()
        .join("; ");

    let top_skills = profile
        .skills
        .iter()
        .take(TOP_SKILLS_COUNT)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    let highest_education = profile
        .education
        .first()
        .map(|e| format!("{} в {}, {}", e.degree, e.institution, e.field))
        .unwrap_or_else(|| "не указано".to_string());

    let languages = profile
        .languages
        .iter()
        .map(|l| format!("{} ({:?})", l.name, l.level))
        .collect::<Vec<_>>()
        .join(", ");

    let target_salary = profile
        .target_salary
        .map(|s| format!("{s} {}", profile.target_currency.as_deref().unwrap_or("RUR")))
        .unwrap_or_else(|| "не указана".to_string());

    let summary = format!(
        "**{}**, {}\n\n**Опыт:** {}\n**Навыки:** {}\n**Образование:** {}\n**Языки:** {}\n**Целевая зарплата:** {}",
        profile.name,
        profile.city,
        if experience_summary.is_empty() {
            "не указан".to_string()
        } else {
            experience_summary
        },
        if top_skills.is_empty() {
            "не указаны".to_string()
        } else {
            top_skills
        },
        highest_education,
        if languages.is_empty() {
            "не указаны".to_string()
        } else {
            languages
        },
        target_salary,
    );

    truncate(&summary, COMPACT_PROFILE_SUMMARY_LIMIT)
}

fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(TRUNCATE_SUFFIX_MARGIN)).collect();
        format!("{truncated}... [truncated]")
    }
}
