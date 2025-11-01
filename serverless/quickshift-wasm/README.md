# quickshift-wasm

Esqueleto de Cloudflare Worker (Rust) para exponer una API mínima `/rutacritica/run` que delegará la lógica de `quickshift` cuando esta sea portada a WASM.

Estrategia recomendada:
1. Extrae la parte "core" de `quickshift` (algoritmo de extracción + planner) a un nuevo crate Rust `quickshift_core` dentro del repo, con `Cargo.toml` apuntando a edition 2021 y sin dependencias no compatibles (sin `tokio`, `actix-web`, `polars`, `plotters`).
2. En `quickshift_core`, conserva dependencias solo wasm-friendly: `petgraph`, `serde`, `serde_json`, `quick-xml` (si se usa en memoria).
3. Agrega un adaptador en `quickshift_wasm` que importe `quickshift_core` y exponga la función HTTP.
 4. Para manejar excel y datos grandes: preprocesa XLSX a JSON fuera del worker (por ejemplo en un job backend) y sube los assets al KV o como assets estáticos del worker. Alternativamente, el worker puede aceptar *bytes* de XLSX en POST (si habilitas la feature `excel`) y el crate `quickshift_wasm` provee funciones para parsear desde buffers en memoria.

Features disponibles
- `excel`: habilita parsing de archivos XLSX desde buffers en memoria usando `calamine`. Útil si quieres enviar el archivo al worker como bytes. No habilitar en WASM si el target no soporta la crate.
- `sql`: habilita un backend SQLite (rusqlite) para persistir tablas preprocesadas. Esto funciona en builds para entornos nativos (no wasm) y sirve para pruebas locales o despliegues en servers.

Compilar y probar localmente:
- Instala target wasm: `rustup target add wasm32-unknown-unknown`
- Instala worker-build (npm): `npm i -g @cloudflare/workers-toolkit` o usar `npx worker-build`
- Construir y empaquetar con wrangler: `npx wrangler dev` (ver `wrangler.toml`)

Notas:
- Este crate es un esqueleto. Puedo ayudarte a hacer el refactor `quickshift -> quickshift_core` y mostrar cambios concretos.
