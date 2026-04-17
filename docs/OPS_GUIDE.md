# BizClaw Operations Guide

## Table of Contents
1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Deployment](#deployment)
4. [Configuration](#configuration)
5. [Monitoring](#monitoring)
6. [Maintenance](#maintenance)
7. [Troubleshooting](#troubleshooting)
8. [Security](#security)

## Overview

BizClaw is a multi-tenant AI agent platform designed for SME businesses in Vietnam. This guide covers operational procedures for production deployments.

### Architecture Overview

```
┌─────────────────────────────────────────────────┐
│                  Load Balancer                   │
└────────────────┬───────────────────────────────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
    ▼            ▼            ▼
┌────────┐  ┌────────┐  ┌────────┐
│Gateway │  │Gateway │  │Gateway │
│ (3001) │  │ (3002) │  │ (3003) │
└───┬────┘  └────┬───┘  └───┬────┘
    │            │          │
    └────────────┼──────────┘
                 │
         ┌───────┴───────┐
         │   Platform    │
         │   (Gateway)    │
         └───────┬───────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
    ▼            ▼            ▼
┌────────┐  ┌────────┐  ┌────────┐
│ BizClaw│  │ BizClaw│  │ BizClaw│
│ Agent  │  │ Agent  │  │ Agent  │
└────────┘  └────────┘  └────────┘
```

## Prerequisites

### System Requirements

- **CPU**: 4+ cores (8 recommended)
- **RAM**: 16GB minimum (32GB recommended)
- **Storage**: 100GB SSD minimum
- **OS**: Ubuntu 22.04 LTS / Debian 12 / macOS 13+
- **Network**: 1Gbps connection

### Required Software

- Docker 24.0+
- Docker Compose 2.20+
- OpenSSL 3.0+
- Git 2.40+

### Required Services

- PostgreSQL 16+ (external)
- Redis 7+ (optional, for caching)
- SMTP Server (for email notifications)

## Deployment

### Quick Start (Docker)

```bash
# Clone the repository
git clone https://github.com/nguyenduchoai/bizclaw-cloud.git
cd bizclaw-cloud

# Copy environment template
cp .env.example .env

# Edit .env with your configuration
nano .env

# Start services
docker-compose up -d

# Verify deployment
curl http://localhost:3001/health
```

### Production Deployment

#### 1. Server Setup

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y \
    apt-transport-https \
    ca-certificates \
    curl \
    gnupg \
    lsb-release

# Add Docker repository
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg

echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# Install Docker
sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

# Add current user to docker group
sudo usermod -aG docker $USER
```

#### 2. SSL Certificate Setup

```bash
# Using Let's Encrypt (recommended)
sudo apt install -y certbot python3-certbot-nginx

# Generate certificate
sudo certbot --nginx -d yourdomain.com -d api.yourdomain.com

# Auto-renewal (should be automatic, but verify)
sudo certbot renew --dry-run
```

#### 3. Deploy with Docker Compose

```bash
# Create production directory
sudo mkdir -p /opt/bizclaw
sudo chown $USER:$USER /opt/bizclaw

# Copy files
cp -r ~/bizclaw-cloud/* /opt/bizclaw/

# Configure
cd /opt/bizclaw
cp .env.example .env
nano .env

# Set proper permissions
chmod 600 .env

# Start services
docker compose -f docker-compose.prod.yml up -d

# Check status
docker compose -f docker-compose.prod.yml ps
```

### Kubernetes Deployment

```bash
# Apply manifests
kubectl apply -f k8s/

# Check pods
kubectl get pods -n bizclaw

# View logs
kubectl logs -n bizclaw deployment/bizclaw-platform -f
```

## Configuration

### Environment Variables

Create `.env` file with the following variables:

```bash
# Application
BIZCLAW_ENV=production
BIZCLAW_LOG_LEVEL=info
BIZCLAW_SECRET_KEY=<generate-32-byte-random-key>

# Database
DATABASE_URL=postgres://bizclaw:password@localhost:5432/bizclaw
DATABASE_POOL_SIZE=20

# Redis (optional)
REDIS_URL=redis://localhost:6379/0

# Security
RATE_LIMIT_PER_MINUTE=60
MAX_REQUEST_SIZE_MB=10

# SMTP
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USER=your-email@gmail.com
SMTP_PASSWORD=your-app-password
SMTP_FROM=bizclaw@yourdomain.com

# AI Providers (optional)
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GOOGLE_AI_API_KEY=...

# Multi-tenancy
MULTI_TENANT_MODE=enabled
MAX_TENANTS=100
```

### Configuration File

Create `/opt/bizclaw/config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 3001
workers = 4
max_connections = 1000

[security]
cors_origins = ["https://yourdomain.com"]
allowed_content_types = ["application/json"]
rate_limit = 60
jwt_secret = "your-jwt-secret"

[database]
pool_size = 20
connection_timeout = 30
idle_timeout = 600

[logging]
level = "info"
format = "json"
output = "stdout"

[features]
ecommerce = true
content = true
office = true
```

## Monitoring

### Health Check

```bash
# Basic health check
curl http://localhost:3001/health

# Detailed health check
curl http://localhost:3001/health/detailed
```

### Metrics

```bash
# Prometheus metrics
curl http://localhost:3001/metrics

# Export for Prometheus
curl http://localhost:3001/metrics | prometheus
```

### Logging

```bash
# View real-time logs
docker compose logs -f bizclaw-platform

# View error logs only
docker compose logs -f bizclaw-platform | grep ERROR

# Export logs
docker compose logs bizclaw-platform > logs.txt
```

### Alerting

Configure alerts for:
- High error rate (>5%)
- Slow response time (>2s)
- High memory usage (>80%)
- Disk space low (<20%)
- Database connection failures

## Maintenance

### Backup

```bash
# Backup database
pg_dump -U bizclaw -h localhost bizclaw > backup_$(date +%Y%m%d_%H%M%S).sql

# Backup configuration
tar -czf config_backup_$(date +%Y%m%d).tar.gz /opt/bizclaw/config.toml

# Automated backup script
/opt/bizclaw/scripts/backup.sh
```

### Updates

```bash
# Pull latest changes
cd /opt/bizclaw
git pull origin main

# Rebuild Docker images
docker compose -f docker-compose.prod.yml build

# Restart services
docker compose -f docker-compose.prod.yml up -d

# Verify
curl http://localhost:3001/health
```

### Log Rotation

Configure log rotation in `/etc/logrotate.d/bizclaw`:

```
/var/log/bizclaw/*.log {
    daily
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 bizclaw bizclaw
    sharedscripts
    postrotate
        docker compose -f /opt/bizclaw/docker-compose.prod.yml reload > /dev/null 2>&1 || true
    endscript
}
```

## Troubleshooting

### Common Issues

#### Service Won't Start

```bash
# Check logs
docker compose logs bizclaw-platform

# Verify configuration
/opt/bizclaw/bizclaw-platform --check-config

# Check database connection
pg_isready -h localhost -U bizclaw

# Restart services
docker compose restart
```

#### High Memory Usage

```bash
# Check container stats
docker stats

# Reduce worker count
# Edit config.toml and set workers = 2

# Restart
docker compose -f docker-compose.prod.yml up -d
```

#### Database Connection Issues

```bash
# Verify database is running
pg_isready -h localhost

# Check connection string
echo $DATABASE_URL

# Test connection
psql $DATABASE_URL -c "SELECT 1;"

# View connection limits
psql $DATABASE_URL -c "SELECT count(*) FROM pg_stat_activity;"
```

#### SSL Certificate Issues

```bash
# Check certificate expiration
openssl s_client -connect yourdomain.com:443 -servername yourdomain.com 2>/dev/null | openssl x509 -noout -dates

# Renew certificate
sudo certbot renew

# Reload nginx
sudo systemctl reload nginx
```

### Performance Optimization

```bash
# Enable query logging
psql $DATABASE_URL -c "ALTER SYSTEM SET log_statement = 'all';"
psql $DATABASE_URL -c "SELECT pg_reload_conf();"

# Analyze slow queries
psql $DATABASE_URL -c "SELECT * FROM pg_stat_statements ORDER BY total_time DESC LIMIT 10;"

# Tune PostgreSQL
# Edit postgresql.conf
max_connections = 100
shared_buffers = 4GB
effective_cache_size = 12GB
maintenance_work_mem = 1GB
work_mem = 64MB
```

## Security

### Firewall Setup

```bash
# Configure UFW
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw enable
```

### Fail2Ban

```bash
# Install fail2ban
sudo apt install -y fail2ban

# Configure
sudo cp /etc/fail2ban/jail.conf /etc/fail2ban/jail.local
sudo nano /etc/fail2ban/jail.local

# Enable Nginx protection
sudo cp /usr/share/doc/fail2ban/config/jail.conf /etc/fail2ban/jail.d/nginx-http-auth.conf

sudo systemctl enable fail2ban
sudo systemctl start fail2ban
```

### Regular Security Updates

```bash
# Create update script
sudo nano /opt/bizclaw/scripts/security-update.sh
```

```bash
#!/bin/bash
# Security update script

set -e

echo "Starting security update..."

# Update system packages
sudo apt update && sudo apt upgrade -y

# Update Docker images
docker compose -f /opt/bizclaw/docker-compose.prod.yml pull

# Run security scan
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
    aquasecurity/trivy:latest image bizclaw/bizclaw:latest

# Restart services with new images
docker compose -f /opt/bizclaw/docker-compose.prod.yml up -d

echo "Security update completed"
```

```bash
# Make executable
sudo chmod +x /opt/bizclaw/scripts/security-update.sh

# Add to crontab
sudo crontab -e
# Add: 0 2 * * 0 /opt/bizclaw/scripts/security-update.sh
```

## Emergency Contacts

- **Developer**: nguyenduchoai@email.com
- **Security Issues**: security@bizclaw.com
- **Business Support**: support@bizclaw.com

## Support

For additional support:
- Documentation: https://docs.bizclaw.com
- GitHub Issues: https://github.com/nguyenduchoai/bizclaw-cloud/issues
- Email: support@bizclaw.com

## License

Copyright © 2024 BizClaw Team. All rights reserved.
