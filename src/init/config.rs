use anyhow::anyhow;
use lettre::transport::smtp::authentication::Credentials;

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
    db_port: Option<u16>,
    db_username: String,
    db_password: String,
    db_name: String,
}

impl DbConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let is_socket_path = std::env::var("DB_HOST")
            .ok()
            .map_or(false, |host| host.starts_with('/'));
    
        if !is_socket_path {
            if let Ok(db_url) = std::env::var("DB_URL") {
                return Self::from_url(&db_url);
            }
        }
    
        let db_type = DbType::Postgres;
        let db_host = std::env::var("DB_HOST")
            .map_err(|_| anyhow!("Environment variable DB_HOST not found"))?;
    
        let db_port = if db_host.starts_with('/') {
            None
        } else {
            Some(
                std::env::var("DB_PORT")
                    .map_err(|_| anyhow!("Environment variable DB_PORT not found"))?
                    .parse::<u16>()?,
            )
        };
    
        let db_username = std::env::var("DB_USERNAME")
            .map_err(|_| anyhow!("Environment variable DB_USERNAME not found"))?;
    
        let db_password = std::env::var("DB_PASSWORD")
            .map_err(|_| anyhow!("Environment variable DB_PASSWORD not found"))?;
            
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
            "postgres" | "psql" | "postgresql" | "pg" => DbType::Postgres,
            "mysql" | "mariadb" | "maria" => DbType::MySql,
            "sqlite" | "sqlite3" => DbType::Sqlite,
            "oracle" | "ora" | "orcl" => DbType::Oracle,
            "mssql" | "microsoftsql" | "sqlserver" => DbType::MsSql,
            _ => {
                return Err(anyhow!(
                    "Unsupported DB; only postgreSQL is supported for now."
                ));
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

        let db_port: Option<u16> = if db_host.starts_with('/') {
            None
        } else if let Some(port_str) = host_and_port_iter.next() {
            Some(port_str.parse::<u16>()?)
        } else {
            match db_type {
                DbType::Postgres => Some(5432),
                DbType::MySql => Some(3306),
                DbType::Sqlite => Some(0),
                DbType::Oracle => Some(1521),
                DbType::MsSql => Some(1433),
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

        // Special handling for Unix Socket
        if scheme == "postgres" && self.db_host.starts_with('/') {
            return Ok(format!(
                "{scheme}://{user}:{pw}@/{db}?host={host}",
                scheme = scheme,
                user = self.db_username,
                pw = self.db_password,
                db = self.db_name,
                host = self.db_host
            ));
        }

        let db_url = format!(
            "{scheme}://{user}:{pw}@{host}{port}/{db}",
            scheme = scheme,
            user = self.db_username,
            pw = self.db_password,
            host = self.db_host,
            port = match self.db_port {
                Some(port) => format!(":{port}"),
                None => String::new(),
            },
            db = self.db_name
        );

        Ok(db_url)
    }
}

pub struct EmailConfig {
    smtp_url: String,
    smtp_username: String,
    smtp_password: String,
}

impl EmailConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let smtp_url = std::env::var("AWS_SES_SMTP_URL")
            .map_err(|_| anyhow!("Environment variable AWS_SES_SMTP_URL not found"))?;
        let smtp_username = std::env::var("AWS_SES_SMTP_USERNAME")
            .map_err(|_| anyhow!("Environment variable AWS_SES_SMTP_USERNAME not found"))?;
        let smtp_password = std::env::var("AWS_SES_SMTP_ACCESS_KEY")
            .map_err(|_| anyhow!("Environment variable AWS_SES_SMTP_ACCESS_KEY not found"))?;

        Ok(EmailConfig {
            smtp_url,
            smtp_username,
            smtp_password,
        })
    }

    pub fn to_creds(&self) -> Credentials {
        Credentials::new(self.smtp_username.clone(), self.smtp_password.clone())
    }

    pub fn get_url(&self) -> String {
        self.smtp_url.clone()
    }
}
