//! Candidate profile data model.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Full candidate profile.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Full name.
    pub name: String,
    /// City / location.
    pub city: String,
    /// Phone number.
    pub phone: String,
    /// Email address.
    pub email: String,
    /// Telegram or other messenger.
    pub telegram: Option<String>,
    /// LinkedIn or personal site.
    pub website: Option<String>,
    /// Citizenship.
    pub citizenship: String,
    /// Work permit status.
    pub work_permit: WorkPermit,
    /// Military status (for male candidates in Russia).
    pub military_status: Option<MilitaryStatus>,
    /// Languages spoken.
    pub languages: Vec<Language>,
    /// Education history.
    pub education: Vec<Education>,
    /// Work experience.
    pub experience: Vec<Experience>,
    /// Technical skills.
    pub skills: Vec<String>,
    /// Soft skills / behavioral profile.
    pub soft_skills: Vec<String>,
    /// Certifications and courses.
    pub certifications: Vec<Certification>,
    /// Professional summary / objective.
    pub summary: String,
    /// Target roles.
    pub target_roles: Vec<String>,
    /// Target salary (optional).
    pub target_salary: Option<i32>,
    /// Target currency.
    pub target_currency: Option<String>,
    /// Ready to relocate.
    pub ready_to_relocate: bool,
    /// Preferred work format (remote, office, hybrid).
    pub work_format: Option<String>,
    /// STAR interview stories.
    pub star_stories: Vec<StarStory>,
    /// Publications, talks, open source.
    pub publications: Vec<String>,
    /// Deal breakers (what to avoid).
    pub deal_breakers: Vec<String>,
    /// What excites you professionally.
    pub motivations: Vec<String>,
    /// Ideal environment description.
    pub ideal_environment: String,
}

/// Work permit status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkPermit {
    #[default]
    Citizen,
    PermanentResident,
    TemporaryResident,
    WorkVisa,
    NoPermit,
}

/// Military service status (Russian-specific).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MilitaryStatus {
    Served,
    Deferred,
    Exempt,
    NotApplicable,
}

/// Language proficiency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Language {
    pub name: String,
    pub level: LanguageLevel,
}

/// CEFR language level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageLevel {
    Native,
    C2,
    C1,
    B2,
    B1,
    A2,
    A1,
}

/// Education entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Education {
    pub institution: String,
    pub degree: String,
    pub field: String,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub description: Option<String>,
}

/// Work experience entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Experience {
    pub company: String,
    pub role: String,
    pub location: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub current: bool,
    pub responsibilities: Vec<String>,
    pub achievements: Vec<String>,
    pub technologies: Vec<String>,
}

/// Certification or course.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Certification {
    pub name: String,
    pub issuer: String,
    pub date: Option<NaiveDate>,
    pub url: Option<String>,
    pub hours: Option<i32>,
}

/// STAR interview story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StarStory {
    pub situation: String,
    pub task: String,
    pub action: String,
    pub result: String,
    pub tags: Vec<String>,
}

impl Profile {
    /// Validate that required fields are present.
    pub fn validate(&self) -> crate::Result<()> {
        if self.name.is_empty() {
            return Err(crate::JobsmithError::ProfileValidation(
                "name is required".to_string(),
            ));
        }
        if self.phone.is_empty() {
            return Err(crate::JobsmithError::ProfileValidation(
                "phone is required".to_string(),
            ));
        }
        if self.email.is_empty() {
            return Err(crate::JobsmithError::ProfileValidation(
                "email is required".to_string(),
            ));
        }
        Ok(())
    }
}
