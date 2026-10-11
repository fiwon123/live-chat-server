# syntax=docker/dockerfile:1.6
#
# Dev image for the Vite dev server. Dependencies are baked into a layer; the
# compose service bind-mounts the source over /workspace and supplies
# node_modules via a named volume, so a rebuild is only needed when
# package*.json changes (`make dev-up` uses `up --build`).
FROM node:24-alpine

WORKDIR /workspace

# Install deps first so this layer is cached until package*.json changes.
COPY package.json package-lock.json ./
RUN npm ci

EXPOSE 5173

# --host makes the dev server reachable from outside the container; --strictPort
# fails instead of silently moving to 5174 when 5173 is taken.
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0", "--port", "5173", "--strictPort"]
