//! An in-memory [`CompanyRepo`].

use std::sync::Mutex;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::CompanyId;

use super::{Company, CompanyDomain, CompanyRepo};
use crate::testing::lock;

/// Keeps companies in a `Vec` and honours the [`CompanyRepo`] contract,
/// including the case-insensitive name order of `list`, which hides merged
/// companies.
#[derive(Debug, Default)]
pub struct FakeCompanyRepo {
    companies: Mutex<Vec<Company>>,
}

#[async_trait]
impl CompanyRepo for FakeCompanyRepo {
    async fn create(&self, company: &Company) -> Result<(), DbError> {
        lock(&self.companies).push(company.clone());
        Ok(())
    }

    async fn find(&self, id: &CompanyId) -> Result<Option<Company>, DbError> {
        let companies = lock(&self.companies);
        Ok(companies.iter().find(|company| company.id == *id).cloned())
    }

    async fn find_by_domain(&self, domain: &CompanyDomain) -> Result<Option<Company>, DbError> {
        let companies = lock(&self.companies);
        Ok(companies
            .iter()
            .find(|company| company.domain.as_ref() == Some(domain))
            .cloned())
    }

    async fn list(&self) -> Result<Vec<Company>, DbError> {
        let mut companies: Vec<Company> = lock(&self.companies)
            .iter()
            .filter(|company| company.merged_into.is_none())
            .cloned()
            .collect();
        companies.sort_by_key(|company| (company.name.to_lowercase(), company.id));
        Ok(companies)
    }

    async fn update(&self, company: &Company) -> Result<bool, DbError> {
        let mut companies = lock(&self.companies);
        let stored = companies.iter_mut().find(|stored| stored.id == company.id);
        Ok(stored.map(|stored| *stored = company.clone()).is_some())
    }

    async fn delete(&self, id: &CompanyId) -> Result<bool, DbError> {
        let mut companies = lock(&self.companies);
        let before = companies.len();
        companies.retain(|company| company.id != *id);
        Ok(companies.len() < before)
    }
}
