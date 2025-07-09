# Rust Backend Template

A production-ready Rust web backend built with Axum, featuring comprehensive blog functionality, user authentication, internationalization, and real-time visitor tracking.

## 🚀 Features

### Core Infrastructure
- **High-performance web server** with Axum and Tokio
- **PostgreSQL integration** with Diesel ORM and async connection pooling
- **TLS/HTTPS support** with automatic HTTP-to-HTTPS redirection
- **Compressed response delivery** with gzip compression
- **Structured logging** with tracing and file output
- **Memory-efficient allocator** using mimalloc

### Authentication & Security
- **JWT-like session management** with UUID-based tokens
- **Secure password hashing** using Argon2
- **Email verification** system with expiring tokens
- **Password reset** functionality
- **API key-based access control**
- **CORS and security middleware**

### Blog Platform
- **Full CRUD operations** for posts and comments
- **Markdown rendering** with comrak
- **Voting system** (upvotes/downvotes) for posts and comments
- **Tag management** and categorization
- **View counting** and engagement metrics
- **Hierarchical comments** with parent-child relationships

### Internationalization (i18n)
- **Multi-language support** with cached string bundles
- **Country/subdivision/language** reference data
- **Bitcode-compressed** i18n data delivery
- **Dynamic cache synchronization**

### Geographic & Analytics
- **IP geolocation** with embedded database
- **Visitor tracking** and analytics
- **OpenStreetMap integration** for location data
- **Real-time visitor board** with coordinate mapping

### Performance & Caching
- **In-memory caching** for posts, countries, and i18n data
- **Lock-free concurrent data structures** using SCC
- **Background job scheduling** for maintenance tasks
- **Efficient binary serialization** with bitcode

## 🏗️ Architecture

```
src/
├── domain/          # Business logic and data models
├── handlers/        # HTTP request handlers
├── routers/         # Route definitions and middleware
├── dto/             # Data transfer objects
├── init/            # Application initialization
├── jobs/            # Background tasks and scheduling
├── util/            # Utility functions and helpers
└── errors/          # Error handling and codes
```

## 🛠️ Tech Stack

- **Framework**: Axum with Tower middleware
- **Database**: PostgreSQL with Diesel ORM
- **Async Runtime**: Tokio
- **Serialization**: Serde + Bitcode
- **Authentication**: Argon2 password hashing
- **Email**: Lettre with SMTP
- **Image Processing**: Image crate with WebP encoding
- **Compression**: Zstd for data, Gzip for HTTP
- **Logging**: Tracing with structured output

## ⚙️ Setup

### Prerequisites
- Rust 1.70+
- PostgreSQL
- OpenSSL/SSL certificates

### Environment Variables
```bash
# Database
DB_URL=postgres://user:pass@localhost/dbname
# or individual components:
DB_HOST=localhost
DB_PORT=5432
DB_USERNAME=user
DB_PASSWORD=pass
DB_NAME=dbname

# Server
HOST_IP=0.0.0.0
HOST_PORT=443
CURR_ENV=local  # or prd

# SSL/TLS
CERT_CHAIN_DIR=/path/to/cert.pem
PRIV_KEY_DIR=/path/to/private.key

# Email (AWS SES)
AWS_SES_SMTP_URL=email-smtp.region.amazonaws.com
AWS_SES_SMTP_USERNAME=your_username
AWS_SES_SMTP_ACCESS_KEY=your_access_key

# Application
APP_NAME_VERSION=rust-backend-v1.0
X_API_KEY=uuid-formatted-api-key
FE_ASSETS_DIR=fe  # Frontend assets directory
```

### Database Setup
```bash
# Install Diesel CLI
cargo install diesel_cli --no-default-features --features postgres

# Run migrations
diesel migration run
```

### Running
```bash
# Development
cargo run

# Production
cargo build --release
./target/release/rust-be-template
```

## 📡 API Endpoints

### Authentication
- `POST /api/auth/signup` - User registration
- `POST /api/auth/login` - User login
- `POST /api/auth/logout` - User logout
- `GET /api/auth/me` - Get current user info
- `POST /api/auth/verify-user-email` - Email verification
- `POST /api/auth/reset-password-request` - Request password reset
- `POST /api/auth/reset-password` - Complete password reset

### Blog
- `GET /api/blog/posts` - List posts (paginated)
- `GET /api/blog/posts/{id}` - Get specific post
- `POST /api/blog/posts` - Create/update post
- `POST /api/blog/{post_id}/vote` - Vote on post
- `POST /api/blog/{post_id}/comment` - Add comment
- `POST /api/blog/{post_id}/{comment_id}/vote` - Vote on comment

### Data & Utilities
- `GET /api/dropdown/country` - List countries
- `GET /api/dropdown/language` - List languages
- `GET /api/geolocate/{ip}` - IP geolocation
- `GET /api/visitor-board` - Visitor analytics

## 🔧 Configuration

The application uses a builder pattern for configuration management and supports both environment variables and database URL formats. SSL/TLS is mandatory for production deployment.

## 📊 Monitoring

Built-in endpoints for health checking and system status:
- `GET /api/healthcheck/server` - Basic health check
- `GET /api/healthcheck/state` - Detailed system status
