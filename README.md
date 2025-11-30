# Pingap Docker Provider

**Automatic service discovery for Pingap reverse proxy based on Docker labels**

A lightweight, high-performance adapter that dynamically manages [Pingap](https://github.com/vicanso/pingap) reverse proxy configuration based on Docker container labels. Inspired by Traefik's label-based configuration.

## Features

- 🔍 **Auto-Discovery**: Automatically detects and configures services from running containers
- 🔄 **Real-time Updates**: Listens to Docker events (start, stop, die) for instant configuration updates
- 🌐 **Multi-Network Support**: Handle containers connected to multiple Docker networks
- 🎯 **Flexible Routing**: Priority-based routing with simplified host/path aliases
- 💪 **Production-Ready**: Stateful tracking, exponential backoff, graceful shutdown
- ⚡ **Lightweight**: Async Rust implementation with minimal resource footprint (<20MB RAM)

## Quick Start

### Using Docker Compose

```yaml
version: '3.8'

services:
  pingap:
    image: pingap/pingap:latest
    ports:
      - "80:80"
      - "6188:6188"
    command: ["-c", "/app/pingap.toml"]

  provider:
    image: pingap-docker-provider:latest
    environment:
      - PINGAP_ADMIN_URL=http://pingap:6188
      - LOG_LEVEL=info
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro

  whoami:
    image: traefik/whoami
    labels:
      - "pingap.enable=true"
      - "pingap.http.host=whoami.local"
```

Then visit `http://whoami.local` to see your service!

## Supported Labels

### Core - Discovery & Networking

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.enable` | **Required**. Enable Pingap routing for this container | `true` |
| `pingap.service.name` | Unique service name (default: container name) | `api-v1` |
| `pingap.service.port` | Explicit port override when container exposes multiple ports | `8080` |
| `pingap.service.address` | Full address override (IP:PORT) | `192.168.1.10:3000` |
| `pingap.docker.network` | Specify which network to use for multi-network containers | `proxy-net` |

### Routing

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.http.rule` | **Explicit routing rule** (advanced) | `Host(\`api.com\`) && PathPrefix(\`/v1\`)` |
| `pingap.http.host` | **Simplified**: Route by hostname | `app.example.com` |
| `pingap.http.paths` | **Simplified**: Route by path (supports comma-separated list) | `/api,/static` |
| `pingap.http.priority` | Rule priority (higher = higher priority) | `10` |

> **Note**: You must provide either `pingap.http.rule`, `pingap.http.host`, or `pingap.http.paths`

### Load Balancing & Upstream

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.upstream.weight` | Server weight for weighted load balancing | `10` |
| `pingap.upstream.strategy` | Load balancing algorithm | `round_robin`, `hash`, `random` |

### Health Checks

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.health_check.path` | Health check endpoint path | `/health` |
| `pingap.health_check.interval` | Time between health checks | `10s` |
| `pingap.health_check.timeout` | Health check timeout | `5s` |

### Middlewares - Path Manipulation

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.middleware.strip_prefix` | Remove path prefix before proxying | `/api` |
| `pingap.middleware.add_prefix` | Add path prefix before proxying | `/v1` |

### Middlewares - Headers & CORS

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.headers.custom_request` | Custom request headers (comma-separated) | `X-Custom: value,X-Another: val` |
| `pingap.headers.custom_response` | Custom response headers (comma-separated) | `X-Served-By: Pingap` |
| `pingap.headers.cors.enable` | Enable basic CORS support | `true` |

### Middlewares - Performance & Compression

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.middleware.compress` | Enable response compression (gzip/brotli) | `true` |

### Security - Rate Limiting

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.middleware.ratelimit.average` | Average requests per second limit | `100` |
| `pingap.middleware.ratelimit.burst` | Burst size for rate limiter | `50` |

### Security - Authentication

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.middleware.basic_auth` | Basic HTTP authentication | `user:hashedpass` |

### Security - Redirects

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.middleware.redirect_scheme` | Force redirect to specific scheme | `https` |
| `pingap.middleware.redirect_regex` | Regex-based redirect | `^http://old/(.*)->https://new/$1` |

### TLS Configuration

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.http.tls.enabled` | Enable TLS for this route | `true` |
| `pingap.tls.redirect` | Automatically redirect HTTP to HTTPS | `true` |
| `pingap.tls.domains` | SAN domains for certificate (comma-separated) | `example.com,api.example.com` |

### Legacy

| Label | Description | Example |
|-------|-------------|---------|
| `pingap.http.middlewares` | Comma-separated middleware names (legacy) | `compress,auth` |

## Label Examples

### Example 1: Simple Web App

```yaml
services:
  webapp:
    image: myapp:latest
    labels:
      - "pingap.enable=true"
      - "pingap.http.host=app.local"
```

Routes all traffic from `app.local` to this container.

### Example 2: API with Path-Based Routing

```yaml
services:
  api:
    image: api:latest
    labels:
      - "pingap.enable=true"
      - "pingap.http.host=api.example.com"
      - "pingap.http.paths=/api,/v1"
      - "pingap.http.priority=10"
```

Routes `api.example.com/api/*` and `api.example.com/v1/*` to this container.

### Example 3: Multi-Network Setup

```yaml
services:
  backend:
    image: backend:latest
    networks:
      - frontend
      - backend
    labels:
      - "pingap.enable=true"
      - "pingap.docker.network=frontend"  # Use IP from frontend network
      - "pingap.service.port=8080"        # Connect to port 8080
      - "pingap.http.host=api.local"

networks:
  frontend:
  backend:
```

### Example 4: Explicit Advanced Rule

```yaml
services:
  service:
    image: service:latest
    labels:
      - "pingap.enable=true"
      - "pingap.http.rule=Host(`domain.com`) && (PathPrefix(`/api`) || PathPrefix(`/v2`))"
      - "pingap.http.priority=20"
      - "pingap.http.middlewares=compress,ratelimit"
```

### Example 5: Load Balancing with Health Checks

```yaml
services:
  api:
    image: api:latest
    deploy:
      replicas: 3
    labels:
      - "pingap.enable=true"
      - "pingap.service.name=api-backend"
      - "pingap.http.host=api.example.com"
      - "pingap.upstream.weight=10"               # Weight for load balancing
      - "pingap.upstream.strategy=round_robin"    # LB strategy
      - "pingap.health_check.path=/healthz"       # Health check endpoint
      - "pingap.health_check.interval=10s"        # Check every 10 seconds
      - "pingap.health_check.timeout=3s"          # 3s timeout
```

### Example 6: Complete Advanced Configuration

```yaml
services:
  secure-api:
    image: api:latest
    labels:
      - "pingap.enable=true"
      - "pingap.service.name=secure-api"
      - "pingap.http.host=api.secure.com"
      - "pingap.http.paths=/api,/v2"
      - "pingap.http.priority=15"
      
      # Middlewares
      - "pingap.middleware.strip_prefix=/api"
      - "pingap.middleware.compress=true"
      - "pingap.headers.cors.enable=true"
      - "pingap.headers.custom_response=X-Powered-By: Pingap,X-API-Version: 2.0"
      
      # Security
      - "pingap.middleware.ratelimit.average=100"
      - "pingap.middleware.ratelimit.burst=50"
      - "pingap.middleware.redirect_scheme=https"
      
      # TLS
      - "pingap.http.tls.enabled=true"
      - "pingap.tls.redirect=true"
      - "pingap.tls.domains=api.secure.com,v2.api.secure.com"
      
      # Health Checks
      - "pingap.health_check.path=/healthz"
      - "pingap.health_check.interval=5s"
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PINGAP_ADMIN_URL` | **Required**. Pingap Admin API URL | - |
| `DOCKER_HOST` | Docker socket path or URL | `/var/run/docker.sock` |
| `LOG_LEVEL` | Logging level (debug, info, warn, error) | `info` |

## How It Works

1. **Initial Sync**: On startup, scans all running containers and applies configurations
2. **Event Monitoring**: Listens to Docker events via streaming API
3. **State Tracking**: Maintains ContainerID→ServiceName mapping for reliable cleanup
4. **API Updates**: Calls Pingap Admin API with exponential backoff retry logic
5. **Graceful Shutdown**: Handles SIGINT/SIGTERM for clean exits

## Building from Source

```bash
cargo build --release
```

### Docker Build

```bash
docker build -t pingap-docker-provider .
```

## Architecture

- **Language**: Rust (async with Tokio)
- **Docker Integration**: `bollard` for Docker API
- **HTTP Client**: `reqwest` with retry logic (`backoff`)
- **Logging**: Structured JSON logs via `tracing`

## Comparison with Traefik

| Feature | Pingap Provider | Traefik |
|---------|----------------|---------|
| Label-based config | ✅ | ✅ |
| Multi-network support | ✅ | ✅ |
| Priority routing | ✅ | ✅ |
| Simplified aliases | ✅ (host/paths) | ❌ |
| Load balancing | ✅ | ✅ |
| Health checks | ✅ | ✅ |
| Path manipulation | ✅ | ✅ |
| Custom headers | ✅ | ✅ |
| CORS | ✅ | ✅ |
| Compression | ✅ | ✅ |
| Rate limiting | ✅ | ✅ |
| Authentication | ✅ | ✅ |
| Redirects | ✅ | ✅ |
| TLS management | ✅ | ✅ |
| Memory footprint | ~15MB | ~80MB |
| Backend | Pingap (Pingora) | Traefik |

## Roadmap

Potential future enhancements:

- Metrics and Prometheus endpoint
- Automatic Let's Encrypt integration
- Advanced middleware chaining
- gRPC support
- Circuit breaker pattern
- Canary deployments support

## License

MIT License
