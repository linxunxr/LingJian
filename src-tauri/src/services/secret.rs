use keyring::Entry;

/// keyring 服务名（应用标识）
const SERVICE_NAME: &str = "com.lingjian.app";

/// 凭证类型：GitHub Token / SCF API Key
#[derive(Debug, Clone, Copy)]
pub enum Secret {
    GithubToken,
    ScfApiKey,
}

impl Secret {
    fn entry_name(&self) -> &'static str {
        match self {
            Secret::GithubToken => "github_token",
            Secret::ScfApiKey => "scf_api_key",
        }
    }
}

/// 保存凭证到系统钥匙串
pub fn set(secret: Secret, value: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, secret.entry_name())
        .map_err(|e| format!("创建钥匙串条目失败: {e}"))?;
    entry.set_password(value).map_err(|e| format!("写入钥匙串失败: {e}"))
}

/// 读取凭证，不存在时返回空串
pub fn get(secret: Secret) -> Result<String, String> {
    let entry = Entry::new(SERVICE_NAME, secret.entry_name())
        .map_err(|e| format!("创建钥匙串条目失败: {e}"))?;
    match entry.get_password() {
        Ok(v) => Ok(v),
        Err(keyring::Error::NoEntry) => Ok(String::new()),
        Err(e) => Err(format!("读取钥匙串失败: {e}")),
    }
}

/// 删除凭证（不存在视为成功）
pub fn delete(secret: Secret) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, secret.entry_name())
        .map_err(|e| format!("创建钥匙串条目失败: {e}"))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("删除钥匙串失败: {e}")),
    }
}
