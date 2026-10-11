# syntax=docker/dockerfile:1.6
#
# Dev image for the Vite dev server. The source is bind-mounted by compose and
# node_modules lives in a named volume; dependencies are installed by
# scripts/dev-entrypoint.sh at start-up so an existing volume can never shadow
# freshly-installed packages.
FROM node:24-alpine

WORKDIR /workspace

EXPOSE 5173

CMD ["sh", "scripts/dev-entrypoint.sh"]
