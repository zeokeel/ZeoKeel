# enterprise-updates (build-time input for the enterprise server image)

This directory is empty in the repository. The release workflow
(`enterprise-image` job) fills it with the three enterprise client installers plus
generated `CLIENT-RELEASE.json` / `CHANNEL.json`, then builds the server image so
enterprise users can download the enterprise client from the deployed server portal
(`/portal`) after login.

The image sets `ZEOKEEL_UPDATE_DIR=/srv/updates` (see `deploy/docker/Dockerfile.server`).
To replace clients without rebuilding the image, mount a host directory and set
`UPDATE_DIR=/data/updates` in `deploy/.env` / compose (docs/09 §5).
