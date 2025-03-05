# portfolio-site-backend

kblue.io backend. Async HTTP server written from scratch in rust.

# Development

It is highly recommended to manually `git clone` the `portfolio-site-infrastructure` repo instead of this one for development of this service to allow for live reload to work in the local k8s dev cluster. The `portfolio-site-infrastructure` repo has a script named `scripts/pull_repos.sh` which will automatically pull all repos associated with kblue.io into the `<infrastructure-git-root>/projects` directory.

In the tilt development environment, live reload of all services is configured to look at the specified repos in the `<infrastructure-git-root>/projects`.

Backend service is exposed on `api.kblue-dev.io:30001` in the dev environment
Postgres is exposed on `localhost:30003` in the dev environment

# Pre-requisites

- Download rust using rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Download psql client (for accessing local database) `sudo apt update && sudo apt install postgresql-client`
