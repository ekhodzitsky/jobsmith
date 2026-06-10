use chrono::NaiveDate;
use jobsmith::error::Result;
use jobsmith::hh::models::*;
use jobsmith::profile::model::*;
use jobsmith::workflow::client::WorkflowClient;
use jobsmith::workflow::state::FitScore;

pub fn dummy_profile() -> Profile {
    Profile {
        schema_version: CURRENT_PROFILE_SCHEMA,
        name: "Иван Иванов".to_string(),
        city: "Москва".to_string(),
        phone: "+79991234567".to_string(),
        email: "ivan@example.com".to_string(),
        telegram: Some("@ivanov".to_string()),
        website: Some("https://ivanov.dev".to_string()),
        citizenship: "РФ".to_string(),
        work_permit: WorkPermit::Citizen,
        military_status: Some(MilitaryStatus::NotApplicable),
        languages: vec![Language {
            name: "Английский".to_string(),
            level: LanguageLevel::C1,
        }],
        education: vec![Education {
            institution: "МГУ".to_string(),
            degree: "Бакалавр".to_string(),
            field: "Программная инженерия".to_string(),
            start_date: NaiveDate::from_ymd_opt(2015, 9, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2019, 6, 30),
            description: None,
        }],
        experience: vec![Experience {
            company: "Яндекс".to_string(),
            role: "Senior Rust Developer".to_string(),
            location: Some("Москва".to_string()),
            start_date: NaiveDate::from_ymd_opt(2019, 7, 1).unwrap(),
            end_date: None,
            current: true,
            responsibilities: vec!["Разработка backend-сервисов".to_string()],
            achievements: vec!["Ускорил сервис в 2x".to_string()],
            technologies: vec!["Rust".to_string(), "Tokio".to_string()],
        }],
        skills: vec!["Rust".to_string(), "Python".to_string()],
        soft_skills: vec!["Коммуникация".to_string()],
        certifications: vec![Certification {
            name: "AWS Certified".to_string(),
            issuer: "Amazon".to_string(),
            date: None,
            url: None,
            hours: None,
        }],
        summary: "Опытный Rust-разработчик".to_string(),
        target_roles: vec!["Senior Rust Developer".to_string()],
        target_salary: Some(500000),
        target_currency: Some("RUR".to_string()),
        ready_to_relocate: false,
        work_format: Some("remote".to_string()),
        star_stories: vec![StarStory {
            situation: "Падение производительности".to_string(),
            task: "Ускорить сервис".to_string(),
            action: "Профилирование и оптимизация".to_string(),
            result: "2x ускорение".to_string(),
            tags: vec!["performance".to_string()],
        }],
        publications: vec![],
        deal_breakers: vec![],
        motivations: vec![],
        ideal_environment: "Удалённая работа в продуктовой компании".to_string(),
    }
}

#[allow(dead_code)]
pub fn dummy_vacancy() -> Vacancy {
    Vacancy {
        id: "123456".to_string(),
        name: "Senior Rust Developer".to_string(),
        description: Some("<p>Разработка на Rust</p>".to_string()),
        salary: Some(Salary {
            from: Some(300000),
            to: Some(500000),
            currency: Some("RUR".to_string()),
            gross: Some(true),
        }),
        employer: Some(Employer {
            id: Some("789".to_string()),
            name: Some("Яндекс".to_string()),
            url: None,
            alternate_url: None,
            logo_urls: None,
            vacancies_url: None,
            trusted: Some(true),
        }),
        area: Some(Area {
            id: Some("1".to_string()),
            name: Some("Москва".to_string()),
            url: None,
            parent_id: None,
        }),
        vacancy_type: Some(VacancyType {
            id: Some("open".to_string()),
            name: Some("Открытая".to_string()),
        }),
        experience: Some(NamedEntity {
            id: Some("between3And6".to_string()),
            name: Some("От 3 до 6 лет".to_string()),
        }),
        schedule: Some(NamedEntity {
            id: Some("remote".to_string()),
            name: Some("Удалённая работа".to_string()),
        }),
        employment: Some(NamedEntity {
            id: Some("full".to_string()),
            name: Some("Полная занятость".to_string()),
        }),
        key_skills: Some(vec![
            NamedEntity {
                id: Some("1".to_string()),
                name: Some("Rust".to_string()),
            },
            NamedEntity {
                id: Some("2".to_string()),
                name: Some("Tokio".to_string()),
            },
        ]),
        published_at: None,
        created_at: None,
        alternate_url: None,
        apply_alternate_url: None,
        address: None,
        snippet: None,
        working_days: None,
        working_time_intervals: None,
        working_time_modes: None,
        accept_temporary: None,
        professional_roles: None,
    }
}

#[allow(dead_code)]
pub fn dummy_vacancy_detail() -> VacancyDetail {
    VacancyDetail {
        base: dummy_vacancy(),
        contacts: None,
        department: None,
        branded_description: None,
        hidden: None,
        response_letter_required: None,
        relocation: None,
        request_id: None,
    }
}

/// Mock AI client for workflow testing.
#[allow(dead_code)]
pub struct MockKimiClient {
    pub fit_score: i32,
    pub evaluation_text: String,
    pub cv_draft: String,
    pub cover_draft: String,
    pub review_text: String,
    pub final_cv: String,
    pub final_cover: String,
}

impl Default for MockKimiClient {
    fn default() -> Self {
        Self {
            fit_score: 75,
            evaluation_text: "Good fit".to_string(),
            cv_draft: "# CV\n\nExperience...".to_string(),
            cover_draft: "Dear Hiring Manager...".to_string(),
            review_text: "Looks good, minor fixes".to_string(),
            final_cv: "# Final CV\n\nPolished...".to_string(),
            final_cover: "Dear Hiring Manager, polished...".to_string(),
        }
    }
}

#[allow(clippy::manual_async_fn)]
impl WorkflowClient for MockKimiClient {
    fn evaluate_fit<'a>(
        &'a mut self,
        _profile: &'a Profile,
        _vacancy: &'a VacancyDetail,
    ) -> impl std::future::Future<Output = Result<(FitScore, String)>> + 'a {
        async move { Ok((FitScore::new(self.fit_score)?, self.evaluation_text.clone())) }
    }

    fn draft_cv<'a>(
        &'a mut self,
        _profile: &'a Profile,
        _vacancy: &'a VacancyDetail,
        _evaluation_text: &'a str,
    ) -> impl std::future::Future<Output = Result<(String, String)>> + 'a {
        async move { Ok((self.cv_draft.clone(), self.cover_draft.clone())) }
    }

    fn review<'a>(
        &'a mut self,
        _profile: &'a Profile,
        _vacancy: &'a VacancyDetail,
        _cv_draft: &'a str,
        _cover_draft: &'a str,
    ) -> impl std::future::Future<Output = Result<String>> + 'a {
        async move { Ok(self.review_text.clone()) }
    }

    fn revise<'a>(
        &'a mut self,
        _profile: &'a Profile,
        _cv_draft: &'a str,
        _cover_draft: &'a str,
        _review: &'a str,
    ) -> impl std::future::Future<Output = Result<(String, String)>> + 'a {
        async move { Ok((self.final_cv.clone(), self.final_cover.clone())) }
    }
}

/// Mock whose responses are RAW markdown routed through the real parsers,
/// unlike `MockKimiClient`, which returns pre-parsed values.
#[allow(dead_code)]
pub struct RawMarkdownMockClient {
    pub evaluation_markdown: String,
    pub revision_markdown: String,
}

#[allow(clippy::manual_async_fn)]
impl WorkflowClient for RawMarkdownMockClient {
    fn evaluate_fit<'a>(
        &'a mut self,
        _profile: &'a Profile,
        _vacancy: &'a VacancyDetail,
    ) -> impl std::future::Future<Output = Result<(FitScore, String)>> + 'a {
        async move {
            let eval = jobsmith::workflow::parser::parse_evaluation(&self.evaluation_markdown)?;
            Ok((FitScore::new(eval.score)?, self.evaluation_markdown.clone()))
        }
    }

    fn draft_cv<'a>(
        &'a mut self,
        _profile: &'a Profile,
        _vacancy: &'a VacancyDetail,
        _evaluation_text: &'a str,
    ) -> impl std::future::Future<Output = Result<(String, String)>> + 'a {
        async move { Ok(("draft cv".to_string(), "draft cover".to_string())) }
    }

    fn review<'a>(
        &'a mut self,
        _profile: &'a Profile,
        _vacancy: &'a VacancyDetail,
        _cv_draft: &'a str,
        _cover_draft: &'a str,
    ) -> impl std::future::Future<Output = Result<String>> + 'a {
        async move { Ok("CRITIQUE: tighten the summary".to_string()) }
    }

    fn revise<'a>(
        &'a mut self,
        _profile: &'a Profile,
        _cv_draft: &'a str,
        _cover_draft: &'a str,
        _review: &'a str,
    ) -> impl std::future::Future<Output = Result<(String, String)>> + 'a {
        async move { jobsmith::workflow::parser::parse_revised(&self.revision_markdown) }
    }
}
