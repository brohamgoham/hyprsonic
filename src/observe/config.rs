use capital_core::EvmAddress;
use serde::Deserialize;
use std::{collections::HashSet, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub max_age_seconds: u64,
    pub accounts: Vec<Account>,
}
#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Account {
    Hyperliquid {
        alias: String,
        address: String,
    },
    Polymarket {
        alias: String,
        address: String,
    },
    Evm {
        alias: String,
        address: String,
        chain_id: u64,
        rpc_url_env: String,
        tokens: Vec<Token>,
    },
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Token {
    pub contract: String,
    pub symbol: String,
    pub decimals: u8,
}
impl Account {
    pub fn alias(&self) -> &str {
        match self {
            Self::Hyperliquid { alias, .. }
            | Self::Polymarket { alias, .. }
            | Self::Evm { alias, .. } => alias,
        }
    }
    pub fn address(&self) -> &str {
        match self {
            Self::Hyperliquid { address, .. }
            | Self::Polymarket { address, .. }
            | Self::Evm { address, .. } => address,
        }
    }
}
impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let data=std::fs::read(path).map_err(|_|"cannot read config; copy config/accounts.example.json to .local/accounts.json and enter public addresses")?;
        if data.len() > 65536 {
            return Err("config exceeds 64 KiB".into());
        }
        let config: Self = serde_json::from_slice(&data).map_err(
            |_| "invalid config schema; check example fields (unknown fields are rejected)",
        )?;
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.accounts.is_empty()
            || self.accounts.len() > 16
            || !(1..=3600).contains(&self.max_age_seconds)
        {
            return Err("configure 1..16 accounts and max_age_seconds 1..3600".into());
        }
        let mut aliases = HashSet::new();
        let mut identities = HashSet::new();
        for account in &self.accounts {
            let alias = account.alias();
            if alias.is_empty()
                || alias.len() > 32
                || !alias
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || !aliases.insert(alias)
            {
                return Err(
                    "aliases must be unique, 1..32 ASCII letters/digits/hyphen/underscore".into(),
                );
            }
            let address = EvmAddress::parse(account.address()).map_err(|_| {
                format!("{alias}: replace placeholder with an actual public account address")
            })?;
            let venue = match account {
                Account::Hyperliquid { .. } => "hl".into(),
                Account::Polymarket { .. } => "poly".into(),
                Account::Evm { chain_id, .. } => format!("evm:{chain_id}"),
            };
            if !identities.insert((venue, address.as_str().to_owned())) {
                return Err("duplicate account identity".into());
            }
            if let Account::Evm {
                chain_id,
                rpc_url_env,
                tokens,
                ..
            } = account
            {
                if *chain_id == 0
                    || tokens.len() > 16
                    || !rpc_url_env.starts_with("HYPRSONIC_")
                    || !rpc_url_env
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                {
                    return Err(format!(
                        "{alias}: invalid chain, token count or RPC environment-variable name"
                    ));
                }
                let mut contracts = HashSet::new();
                for token in tokens {
                    let address =
                        EvmAddress::parse(&token.contract).map_err(|_| "invalid token contract")?;
                    if token.decimals > 38
                        || token.symbol.is_empty()
                        || token.symbol.len() > 16
                        || !token
                            .symbol
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_')
                        || !contracts.insert(address.as_str().to_owned())
                    {
                        return Err("invalid or duplicate token configuration".into());
                    }
                }
            }
        }
        Ok(())
    }
}
