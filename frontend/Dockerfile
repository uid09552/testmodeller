# ── Build ─────────────────────────────────────────────────────────────────────
FROM node:24-alpine AS builder

WORKDIR /build

# Lockfile-only install layer, so source edits do not re-run npm ci.
COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npm run build -- --configuration production

# ── Runtime ───────────────────────────────────────────────────────────────────
FROM nginx:1.27-alpine AS runtime

RUN rm -rf /usr/share/nginx/html/*

# Angular's application builder emits the browser bundle under dist/<project>/browser.
COPY --from=builder /build/dist/testmodeller-ui/browser /usr/share/nginx/html

# The nginx image runs envsubst over /etc/nginx/templates/*.template at start.
COPY nginx.conf.template /etc/nginx/templates/default.conf.template

# Where the API lives. Override to point at a backend outside compose.
ENV BACKEND_ORIGIN=http://backend:8080
# Opt in to the image populating NGINX_LOCAL_RESOLVERS from /etc/resolv.conf;
# without this flag its entrypoint script returns early and the variable is unset.
ENV NGINX_ENTRYPOINT_LOCAL_RESOLVERS=1
# Restrict substitution to these names so nothing else in the template is touched.
ENV NGINX_ENVSUBST_FILTER='(BACKEND_ORIGIN|NGINX_LOCAL_RESOLVERS)'

EXPOSE 80

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -q --spider http://127.0.0.1/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
