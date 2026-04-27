use serde::{Deserialize, Serialize};

use crate::{FunctionMeta, RuneError};

pub const RUNE_STATE_DIR: &str = ".rune";
pub const FUNCTIONS_DIR: &str = ".rune/functions";
pub const DEPLOYMENTS_MANIFEST_PATH: &str = ".rune/deployments.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploymentManifest {
    pub functions: Vec<FunctionMeta>,
}

impl DeploymentManifest {
    pub fn upsert(&mut self, meta: FunctionMeta) -> Result<(), RuneError> {
        if self
            .functions
            .iter()
            .any(|existing| existing.route == meta.route && existing.id != meta.id)
        {
            return Err(RuneError::DuplicateRoute {
                route: meta.route.clone(),
            });
        }

        if let Some(existing) = self
            .functions
            .iter_mut()
            .find(|existing| existing.id == meta.id)
        {
            *existing = meta;
        } else {
            self.functions.push(meta);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_replaces_existing_function_with_same_id() {
        let mut manifest = DeploymentManifest {
            functions: vec![FunctionMeta {
                id: "hello".into(),
                subdomain: None,
                route: "/hello".into(),
                wasm_path: ".rune/functions/hello.wasm".into(),
            }],
        };

        manifest
            .upsert(FunctionMeta {
                id: "hello".into(),
                subdomain: None,
                route: "/v2/hello".into(),
                wasm_path: ".rune/functions/hello-v2.wasm".into(),
            })
            .unwrap();

        assert_eq!(manifest.functions.len(), 1);
        assert_eq!(manifest.functions[0].route, "/v2/hello");
        assert_eq!(
            manifest.functions[0].wasm_path,
            ".rune/functions/hello-v2.wasm"
        );
    }

    #[test]
    fn upsert_rejects_duplicate_route_for_different_function() {
        let mut manifest = DeploymentManifest {
            functions: vec![FunctionMeta {
                id: "hello".into(),
                subdomain: None,
                route: "/hello".into(),
                wasm_path: ".rune/functions/hello.wasm".into(),
            }],
        };

        let err = manifest
            .upsert(FunctionMeta {
                id: "hello-2".into(),
                subdomain: None,
                route: "/hello".into(),
                wasm_path: ".rune/functions/hello-2.wasm".into(),
            })
            .unwrap_err();

        match err {
            RuneError::DuplicateRoute { route } => assert_eq!(route, "/hello"),
            other => panic!("expected duplicate route error, got {other}"),
        }
    }
}
