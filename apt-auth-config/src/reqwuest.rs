use reqwest_middleware::RequestInitialiser;
use std::sync::Arc;

use crate::AuthConfig;

pub struct AuthMiddleware {
    config: Arc<AuthConfig>,
}

impl AuthMiddleware {
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl RequestInitialiser for AuthMiddleware {
    fn init(&self, req: reqwest_middleware::RequestBuilder) -> reqwest_middleware::RequestBuilder {
        let url = req
            .try_clone()
            .unwrap()
            .build()
            .ok()
            .map(|r| r.url().clone());

        if let Some(url) = url
            && let Some(auth) = self.config.find(&url)
        {
            let login = &auth.login;
            let passwd = &auth.password;

            req.basic_auth(login, Some(passwd))
        } else {
            req
        }
    }
}
