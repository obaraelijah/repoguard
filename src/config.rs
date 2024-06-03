use std::rc::Rc;

use serde::Deserialize;

pub(crate) struct Repository {
    owner: String,
    repository: String,   
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub default_owner: Rc<str>,
    pub default_repo: Rc<str>,
}

pub async fn query(
    def_owner: Rc<str>,
    defo_repo: Rc<str>,
) {
    let default = Repository {
        owner: def_owner.to_string(),
        repository: defo_repo.to_string(),
    };  
}

