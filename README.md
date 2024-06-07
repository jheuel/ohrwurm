<p align="center">
  <img alt="Light" src="./docs/logo.png" width="20%">
</p>
<p align="center">
  Ohrwurm
</p>

# Deployment
To deploy Ohrwurm with Docker, you can use the provided Docker image:
```bash
docker run -d \
  --name ohrwurm \
  --restart unless-stopped \
  -e DISCORD_TOKEN=YOUR_DISCORD_BOT_TOKEN \
  -e DISCORD_APP_ID=YOUR_DISCORD_APP_ID \
  -e ADMIN=YOUR_DISCORD_USER_ID \
  jheuel/ohrwurm:latest
```

Alternatively, you can create a `docker-compose.yml` file:
```yaml
services:
  ohrwurm:
    container_name: ohrwurm
    image: jheuel/ohrwurm:latest
    restart: unless-stopped
    env:
      - DISCORD_TOKEN=YOUR_DISCORD_BOT_TOKEN
      - DISCORD_APP_ID=YOUR_DISCORD_APP_ID
      - ADMIN=YOUR_DISCORD_USER_ID
```
and then run the image with `docker compose up`.
