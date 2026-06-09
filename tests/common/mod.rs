use chrono::NaiveDate;
use jobsmith::hh::models::*;
use jobsmith::profile::model::*;

pub fn dummy_profile() -> Profile {
    Profile {
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
        employer: Employer {
            id: "789".to_string(),
            name: "Яндекс".to_string(),
            url: None,
            alternate_url: None,
            logo_urls: None,
            vacancies_url: None,
            trusted: Some(true),
        },
        area: Some(Area {
            id: "1".to_string(),
            name: "Москва".to_string(),
            url: None,
            parent_id: None,
        }),
        vacancy_type: Some(VacancyType {
            id: "open".to_string(),
            name: "Открытая".to_string(),
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
        extra: serde_json::Value::Null,
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
