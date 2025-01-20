use anyhow::anyhow;
use std::net::IpAddr;

#[repr(u8)]
enum DbType {
    Postgres,
    MySql,
    Sqlite,
    Oracle,
    MsSql,
}

pub struct DbConfig {
    db_type: DbType,
    db_host: String,
    db_port: u16,
    db_username: String,
    db_password: String,
    db_name: String,
}

impl DbConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        if let Ok(db_url) = std::env::var("DB_URL") {
            return Self::from_url(&db_url);
        }

        let db_type = DbType::Postgres; // Default to Postgres if not specified in URL
        let db_host = std::env::var("DB_HOST")
            .map_err(|_| anyhow!("Environment variable DB_HOST not found"))?;
        let db_port = std::env::var("DB_PORT")
            .map_err(|_| anyhow!("Environment variable DB_PORT not found"))?
            .parse::<u16>()?;
        let db_username = std::env::var("DB_USERNAME")
            .map_err(|_| anyhow!("Environment variable DB_USERNAME not found"))?;
        let db_password = std::env::var("DB_PASSWORD")
            .map_err(|_| anyhow!("Environment variable DB_PW not found"))?;
        let db_name = std::env::var("DB_NAME")
            .map_err(|_| anyhow!("Environment variable DB_NAME not found"))?;

        Ok(DbConfig {
            db_type,
            db_host,
            db_port,
            db_username,
            db_password,
            db_name,
        })
    }

    pub fn from_url(url: &str) -> anyhow::Result<Self> {
        let separator_pos = url
            .find("://")
            .ok_or_else(|| anyhow!("Invalid URL format"))?;
        let scheme = &url[..separator_pos];
        let rest = &url[separator_pos + 3..];

        let db_type = match scheme.trim().to_lowercase().as_ref() {
            "postgres" | "psql" | "postgresql" => DbType::Postgres,
            "mysql" => DbType::MySql,
            "sqlite" => DbType::Sqlite,
            "oracle" => DbType::Oracle,
            "mssql" | "microsoftsql" => DbType::MsSql,
            _ => {
                return Err(anyhow!(
                    "Unsupported DB; only postgreSQL is supported for now."
                ))
            }
        };

        let mut credentials_and_host = rest.split('@');
        let credentials = credentials_and_host
            .next()
            .ok_or_else(|| anyhow!("Missing credentials"))?;
        let host_and_path = credentials_and_host
            .next()
            .ok_or_else(|| anyhow!("Missing host and path"))?;

        let mut credentials_iter = credentials.split(':');
        let db_username = credentials_iter.next().unwrap_or("").to_string();
        let db_password = credentials_iter.next().unwrap_or("").to_string();

        let mut host_and_path_iter = host_and_path.split('/');
        let host_and_port = host_and_path_iter
            .next()
            .ok_or_else(|| anyhow!("Missing host"))?;
        let db_name = host_and_path_iter.next().unwrap_or("").to_string();

        let mut host_and_port_iter = host_and_port.split(':');
        let db_host = host_and_port_iter
            .next()
            .ok_or_else(|| anyhow!("Missing host"))?;

        let db_port = if let Some(port_str) = host_and_port_iter.next() {
            port_str.parse::<u16>()?
        } else {
            match db_type {
                DbType::Postgres => 5432,
                DbType::MySql => 3306,
                DbType::Sqlite => 0,
                DbType::Oracle => 1521,
                DbType::MsSql => 1433,
            }
        };

        Ok(DbConfig {
            db_type,
            db_host: db_host.to_owned(),
            db_port,
            db_username,
            db_password,
            db_name,
        })
    }

    pub fn to_url(&self) -> anyhow::Result<String> {
        let scheme = match self.db_type {
            DbType::Postgres => "postgres",
            DbType::MySql => "mysql",
            DbType::Sqlite => "sqlite",
            DbType::Oracle => "oracle",
            DbType::MsSql => "mssql",
        };

        Ok(format!(
            "{}://{}:{}@{}:{}/{}",
            scheme, self.db_username, self.db_password, self.db_host, self.db_port, self.db_name
        ))
    }
}
