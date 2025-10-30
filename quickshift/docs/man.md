# quickshift — Manual de uso y notas técnicas

Este documento resume cómo el crate `quickshift` lee los archivos Excel, qué nuevas piezas de datos se agregaron (porcentaje de aprobados / dificultad), la API REST expuesta y recomendaciones para mejorar la robustez.

## Resumen corto

- El sistema lee 3 tipos de artefactos Excel:
    - Malla curricular (ej. `MallaCurricular2010.xlsx`, `MiMalla.xlsx`)
    - Oferta académica (ej. `Oferta Academica ...xlsx`)
    - Porcentaje de aprobados (nuevo) `PorcentajeAPROBADOS2025-1.xlsx`
- Se añadió un campo `dificultad: Option<f64>` en la estructura `RamoDisponible` que contiene el porcentaje de aprobados (0.0–100.0). Valores bajos indican más difícil.
- El servidor expone `/help` (GET) y `/solve` (POST JSON). `/solve` ejecuta la canalización y devuelve hasta 10 soluciones y conteo de documentos leídos.

## Formato esperado de los Excel

- Malla curricular (leer con `leer_malla_excel`):
    - Primera hoja por defecto.
    - Columnas esperadas (orden típico): `Codigo`, `Nombre`, `Correlativo`, `Holgura`, `Critico`.
    - Las columnas pueden tener distintos tipos (string, float, int); la lectura intenta normalizar.

- Oferta académica (leer con `leer_oferta_academica_excel`):
    - Se intenta leer hojas candidatas (`Mi Malla`, `MiMalla`, etc.) y en último caso la primera hoja del workbook.
    - Columnas esperadas: `Codigo`, `Nombre`, `Seccion`, `Horario`, `Profesor`, `CodigoBox`.
    - Si `CodigoBox` no está presente o es ruido, el parser intenta derivarlo desde `Codigo` (por ejemplo tomando la parte antes de un `-`).
    - Si calamine falla en la lectura, se utiliza un fallback que abre el `.xlsx` como ZIP y parsea `xl/worksheets/sheetN.xml` y `xl/sharedStrings.xml`.

- Porcentajes (`leer_porcentajes_aprobados`):
    - Se espera una hoja con al menos dos columnas: `Codigo` (columna 0) y `Porcentaje` (columna 1).
    - El campo `Porcentaje` puede venir como `"78%"`, `"78,5"`, `78.5`, etc. La función elimina `%` y convierte comas a punto antes de parsear como `f64`.
    - La función intenta leer con calamine; si falla, usa el fallback ZIP/XML.

## Dónde se almacena la dificultad

- `RamoDisponible` ahora incluye `dificultad: Option<f64>`. Si el archivo de porcentajes contiene una fila para el código de un ramo (o `codigo_ref`), se asigna dicho valor.

## Interpretación de `dificultad`

- Por convención aquí usada: dificultad inversa respecto al porcentaje de aprobados.
    - Porcentaje bajo (ej. 20.0) → más difícil.
    - Porcentaje alto (ej. 85.0) → más fácil.
- Actualmente el campo se almacena y se imprime en la salida de diagnóstico. Aún no se aplica automáticamente a la heurística de prioridades — si deseas que influya en la selección de soluciones, especifica la política (prefiere ramos fáciles/difíciles o solo informar).

## API REST

- GET /help — devuelve un JSON con ejemplo de `InputParams` y las mallas soportadas.
- POST /solve — acepta un JSON con parámetros (correo, `ramos_pasados` por código, `ramos_prioritarios`, `horarios_preferidos`, opcional `malla`) y responde con:
    - `documentos_leidos`: número de documentos (malla/oferta/porcentajes) leídos correctamente.
    - `soluciones_count`: cantidad de soluciones devueltas.
    - `soluciones`: arreglo con hasta 10 soluciones (cada una incluye `secciones` y `total_score`).

Ejemplo JSON (en `GET /help` también aparece):

```json
{
    "email": "alumno@ejemplo.com",
    "ramos_pasados": ["CIT3313", "CIT3211"],
    "ramos_prioritarios": ["CIT3413"],
    "horarios_preferidos": ["LU 08:30"],
    "malla": "MallaCurricular2020.xlsx"
}
```

## Notas sobre normalización y problemas conocidos

- Las hojas de `Oferta` en el mundo real a menudo tienen columnas con contenido mixto (RUTs, horarios, códigos no canónicos). Esto provoca que `codigo_box` no coincida con las claves en la malla y por eso el motor encuentre cero secciones.
- Recomendaciones para mejorar coincidencia:
    - Normalizar códigos a mayúsculas y sin espacios/guiones antes de crear claves.
    - Construir un mapa de alias (ej. "CIT3313-1" -> "CIT3313", o patrones con sufijos/prefijos) si el departamento usa variantes.
    - Validar en un paso previo cuántas secciones se emparejan y loggear ejemplos para crear reglas de limpieza.

## Próximos pasos sugeridos

1. Implementar normalización robusta de `codigo_box` (regex + uppercase + eliminar sufijos como `-A`, `SEC1`).
2. Decidir política de uso de `dificultad` en la heurística y aplicarla (ej. penalizar ramos con % bajo).
3. Añadir tests unitarios que verifiquen la carga de porcentajes y la propagación a `RamoDisponible`.
4. Añadir una opción en la API para subir el archivo de porcentajes o indicar su path en la petición.

## Ubicación de archivos y funciones clave

- Lectura malla: `src/excel/mod.rs` -> `leer_malla_excel`
- Lectura oferta: `src/excel/mod.rs` -> `leer_oferta_academica_excel` (usa fallback ZIP/XML)
- Lectura porcentajes: `src/excel/mod.rs` -> `leer_porcentajes_aprobados`
- Estructuras: `src/models/mod.rs` (campo `dificultad` añadido)
- Integración y heurísticas: `src/algorithms/mod.rs` (se propaga dificultad en `get_ramo_critico`)
- API REST: `src/server.rs` (/help, /solve)

---

Si quieres que aplique alguna política concreta de uso de `dificultad` en la puntuación de soluciones o que añada tests que cubran casos reales del `PorcentajeAPROBADOS2025-1.xlsx`, dime cuál prefieres y lo implemento ahora. También puedo añadir un script pequeño que muestre los ramos que no fueron emparejados para facilitar crear reglas de normalización.
# quickshift(1) — Manual breve del crate quickshift

NAME
----
quickshift — Generador de horarios: extracción desde Excel y algoritmo de clique ponderado

SYNOPSIS
--------
# quickshift(1) — Manual breve del crate quickshift

NAME
----
quickshift — Generador de horarios: extracción desde Excel y algoritmo de clique ponderado

SYNOPSIS
--------
Desde la raíz del directorio `quickshift`:
- Revisar:    `cargo check`
- Compilar:   `cargo build`
- Ejecutar:   `cargo run`
- Release:    `cargo build --release`

DESCRIPTION
-----------
quickshift es un crate Rust que implementa un flujo para generar horarios académicos:
1. Extrae datos desde archivos Excel (mallas, oferta, mallas del alumno).
2. Construye estructuras internas (Seccion, RamoDisponible, etc.).
3. Ejecuta un algoritmo de clique ponderado para proponer conjuntos de secciones coherentes.

Estructura del proyecto (directorio por directorio)
---------------------------------------------------

Raíz (quickshift/)
- Cargo.toml / Cargo.lock: metadatos y dependencias (calamine, petgraph, chrono, ...).
- MiMalla.xlsx, OfertaAcademica2024.xlsx: ejemplos de datos para pruebas.
- target/: artefactos binarios y compilación generada por Cargo.

src/
- lib.rs
   - Exporta módulos públicos del crate; permite usar quickshift como librería.
- main.rs
   - Punto de entrada del binario.
   - Orquesta el flujo: obtener ramos críticos → extraer datos → ejecutar clique → mostrar resumen.

src/models/
- Contiene las definiciones de dominio:
   - Seccion: representa una sección concreta (código, sección, horario, profesor, prioridad, ...).
   - RamoDisponible: metadata por ramo (aprobados, prioridad, referencia, ...).
   - PertNode y otras estructuras auxiliares.
- Usar este módulo para añadir campos o anotaciones de serialización.

src/excel/
- Abstracción de lectura de Excel usando la crate `calamine`.
- Helpers para abrir workbooks, leer hojas y convertir filas a valores tipados.
- Lugar ideal para validar esquemas de hoja y normalizar columnas.

src/algorithms/
- Núcleo algorítmico:
   - Preparación de datos y utilidades.
   - `get_clique_max_pond`: algoritmo principal, ahora devuelve resultados como datos (Vec de soluciones).
   - Funciones de fallback y simulación (útiles para tests).
- Recomendación: envolver el `Vec` anidado actual en structs (`Solution`, `Entry`) para API más clara.

src/rutacritica/
- Port literal del proyecto RutaCritica (script Python):
   - extract.rs: traducción fiel del parser Python; usa calamine y propaga errores con `Result`.
   - clique.rs: adaptador entre extractor y `get_clique_max_pond`.
   - ruta.rs / mod.rs: orquestador `run_ruta_critica()` que devuelve `Result`.
- Mantener este módulo como referencia para futuras mejoras.

ERROR HANDLING
--------------
- La extracción ahora propaga errores (`Result<..., Box<dyn Error>>`).
- main deberia capturar y mostrar errores legibles al usuario.
- Recomendación: definir un enum de errores propio para mensajes más claros.

COMANDOS ÚTILES
--------------
- Verificación rápida: `cargo check`
- Compilar + generar binario: `cargo build`
- Ejecutar directamente: `cargo run`
- Ejecutar con release: `cargo run --release`
- Ejecutar tests (cuando estén añadidos): `cargo test`

ARCHIVOS DE ENTRADA
-------------------
- Ejemplos incluidos:
   - `MiMalla.xlsx`
   - `OfertaAcademica2024.xlsx`
- Los nombres y rutas pueden parametrizarse en el futuro (usar `clap`/`structopt`).

Nota: Cuando envíes `ramos_pasados` al endpoint `/solve`, deben ser códigos de ramo (por ejemplo `CIT3313`) tal como aparecen en la fila/columna "Asignatura" del archivo `OfertaAcademica2024.xlsx`.

Parámetro `malla` (opcional):
- Puedes indicar qué malla curricular usar pasando el nombre del archivo en el campo `malla` del payload JSON. Archivos disponibles en el repo:
    - `MallaCurricular2010.xlsx`
    - `MallaCurricular2018.xlsx`
    - `MallaCurricular2020.xlsx`

PRÓXIMOS PASOS RECOMENDADOS
---------------------------
1. Definir tipos de salida:
    - Crear `Solution` y `SectionEntry` para reemplazar `Vec<Vec<(Seccion, i32)>>`.
2. Mejorar los errores:
    - Replazar `Box<dyn Error>` por `enum Error` propio y mensajes claros.
3. Tests de integración:
    - Añadir tests que ejecuten `extract_data` sobre los Excel incluidos y validen outputs básicos.
4. CLI y configuración:
    - Añadir flags para pasar rutas XLSX, modo verbose y salida JSON.
5. Limpieza:
    - Eliminar `#[allow(dead_code)]` usando o removiendo código no usado.
6. Documentación:
    - Comentar API pública y documentar formato de hojas Excel esperadas.

EJEMPLO DE USO RÁPIDO
--------------------
1. Desde la carpeta `quickshift`:
    - `cargo run`
2. Salida esperada:
    - Resumen en consola con número de secciones procesadas y cantidad de soluciones devueltas por el algoritmo.

LICENSE
-------
Revisa `Cargo.toml` para dependencias y licencias. Mantener compatibilidad con licencias de crates usados.

CONTACTO
-------
Este documento resume la arquitectura y puntos de mejora. Para cambios automáticos o parches, ejecutar las acciones sugeridas en el módulo correspondiente.

---

Documento generado automáticamente por la herramienta de desarrollo. Para cambios, edita `man.md` en la raíz del crate `quickshift`.