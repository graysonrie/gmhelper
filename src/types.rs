use serde::{Deserialize, Serialize};

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameMakerVersion {
    #[serde(rename = "beta")]
    GMBETA,
    #[serde(rename = "lts2026")]
    GMLTS2026,
}

#[allow(unused)]
impl GameMakerVersion {
    pub fn from_use_gm_beta(use_gm_beta: bool) -> Self {
        if use_gm_beta {
            Self::GMBETA
        } else {
            Self::GMLTS2026
        }
    }

    pub fn is_beta(self) -> bool {
        matches!(self, Self::GMBETA)
    }
}
