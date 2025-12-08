# 📚 API ENDPOINTS: Consulta de Cursos de Malla

**Documentación Técnica de Endpoints de Cursos**

---

## 📋 Índice Rápido

| Aspecto | Descripción |
|---------|-------------|
| **Endpoints** | 3 endpoints REST para consulta de cursos |
| **Funcionalidad** | Obtener cursos de malla y calcular elegibilidad |
| **Método** | GET para listados, POST para recomendaciones |
| **Formato** | JSON request/response |
| **Performance** | O(n) con HashMap optimizado |


Se implementaron 3 endpoints REST que permiten:

1. **Obtener todos los cursos de una malla** - Lista completa de cursos con metadata
2. **Obtener cursos por semestre** - Filtrar cursos de un semestre específico
3. **Obtener cursos elegibles** - Calcular qué cursos puede tomar un estudiante dado su historial

Estos endpoints utilizan el **Mapeo Maestro optimizado** para lectura eficiente de mallas curriculares.

---

### 1. GET `/api/mallas/{malla_id}/cursos`

**Descripción:** Obtiene todos los cursos de una malla curricular completa.

#### Request

```http
GET /api/mallas/MallaCurricular2020.xlsx/cursos?sheet=Malla%202020
```

**Path Parameters:**
- `malla_id` (string, requerido): Nombre del archivo de malla curricular
  - Ejemplos: `MallaCurricular2020.xlsx`, `MallaCurricular2018.xlsx`

**Query Parameters:**
- `sheet` (string, opcional): Nombre de la hoja dentro del archivo Excel
  - Ejemplo: `Malla 2020`, `Electivos`
  - Si se omite, se usa la primera hoja disponible

#### Response

**Status:** `200 OK`

```json
{
  "malla": "MallaCurricular2020.xlsx",
  "cursos": [
    {
      "id": 1,
      "nombre": "Cálculo I",
      "codigo": "CBM1001",
      "semestre": 1,
      "requisitos_ids": [],
      "electivo": false,
      "dificultad": 0.65,
      "numb_correlativo": 1,
      "critico": true
    },
    {
      "id": 2,
      "nombre": "Álgebra Lineal",
      "codigo": "CBM1002",
      "semestre": 1,
      "requisitos_ids": [],
      "electivo": false,
      "dificultad": 0.58,
      "numb_correlativo": 2,
      "critico": false
    },
    {
      "id": 15,
      "nombre": "Cálculo II",
      "codigo": "CBM1003",
      "semestre": 2,
      "requisitos_ids": [1],
      "electivo": false,
      "dificultad": 0.72,
      "numb_correlativo": 1,
      "critico": true
    }
  ]
}
```

**Campos del Curso:**
- `id` (integer): Identificador único del curso en la malla
- `nombre` (string): Nombre completo del curso
- `codigo` (string): Código del curso (ej: CBM1001, CIT3313)
- `semestre` (integer|null): Semestre recomendado (1-10), null si no aplica
- `requisitos_ids` (array): Lista de IDs de cursos prerequisitos
- `electivo` (boolean): Indica si el curso es electivo
- `dificultad` (float|null): Índice de dificultad (0.0-1.0), null si no disponible
- `numb_correlativo` (integer): Número correlativo dentro del semestre
- `critico` (boolean): Indica si el curso está en la ruta crítica

---

### 2. GET `/api/mallas/{malla_id}/semestres/{semestre}/cursos`

**Descripción:** Obtiene los cursos de un semestre específico de una malla curricular.

#### Request

```http
GET /api/mallas/MallaCurricular2020.xlsx/semestres/3/cursos?sheet=Malla%202020
```

**Path Parameters:**
- `malla_id` (string, requerido): Nombre del archivo de malla curricular
- `semestre` (integer, requerido): Número de semestre (típicamente 1-10)

**Query Parameters:**
- `sheet` (string, opcional): Nombre de la hoja dentro del archivo Excel

#### Response

**Status:** `200 OK`

```json
{
  "malla": "MallaCurricular2020.xlsx",
  "semestre": 3,
  "cursos": [
    {
      "id": 20,
      "nombre": "Estructuras de Datos",
      "codigo": "CIT2111",
      "semestre": 3,
      "requisitos_ids": [5, 8],
      "electivo": false,
      "dificultad": 0.68,
      "numb_correlativo": 1,
      "critico": true
    },
    {
      "id": 21,
      "nombre": "Bases de Datos",
      "codigo": "CIT2112",
      "semestre": 3,
      "requisitos_ids": [5],
      "electivo": false,
      "dificultad": 0.55,
      "numb_correlativo": 2,
      "critico": false
    }
  ]
}
```

**Nota:** Si el semestre no tiene cursos asignados, se devuelve un array vacío `[]`.

---

### 3. POST `/api/cursos/recomendados`

**Descripción:** Calcula qué cursos puede tomar un estudiante dado su historial de ramos aprobados. Este endpoint evalúa los prerequisitos y devuelve solo los cursos elegibles.

#### Request

```http
POST /api/cursos/recomendados
Content-Type: application/json
```

```json
{
  "malla_id": "MallaCurricular2020.xlsx",
  "ramos_aprobados": [
    "CBM1001",
    "CBM1002",
    "Programación I",
    "CIT2111"
  ],
  "sheet": "Malla 2020"
}
```

**Campos del Request:**
- `malla_id` (string, requerido): Nombre del archivo de malla curricular
- `ramos_aprobados` (array, opcional): Lista de cursos ya aprobados
  - Puede contener **códigos** (ej: `CBM1001`) o **nombres** (ej: `Programación I`)
  - El sistema normaliza nombres para comparación (ignora mayúsculas/acentos)
  - Array vacío `[]` devuelve cursos de primer semestre
- `sheet` (string, opcional): Nombre de la hoja dentro del archivo Excel

#### Response

**Status:** `200 OK`

```json
{
  "malla": "MallaCurricular2020.xlsx",
  "total_elegibles": 5,
  "cursos": [
    {
      "id": 15,
      "nombre": "Cálculo II",
      "codigo": "CBM1003",
      "semestre": 2,
      "requisitos_ids": [1],
      "electivo": false,
      "dificultad": 0.72,
      "numb_correlativo": 1,
      "critico": true
    },
    {
      "id": 16,
      "nombre": "Física I",
      "codigo": "FIS1001",
      "semestre": 2,
      "requisitos_ids": [1],
      "electivo": false,
      "dificultad": 0.63,
      "numb_correlativo": 2,
      "critico": false
    }
  ]
}
```

**Campos de la Respuesta:**
- `malla` (string): Nombre de la malla consultada
- `total_elegibles` (integer): Cantidad total de cursos elegibles
- `cursos` (array): Lista de cursos que el estudiante puede tomar

**Lógica de Elegibilidad:**

Un curso es **elegible** si:
1. ✅ Todos sus prerequisitos están en `ramos_aprobados`
2. ✅ El curso mismo NO está en `ramos_aprobados` (no repetir cursos)

```

### Flujo de Datos

#### 1. Obtener Todos los Cursos

```rust
// Pseudocódigo simplificado
async fn cursos_todos_handler(malla_id: String) -> Response {
    // 1. Cargar malla en HashMap
    let map = load_malla_map(&malla_id, sheet)?;
    
    // 2. Convertir a DTOs
    let mut cursos: Vec<CursoDto> = map
        .values()
        .map(ramo_to_dto)
        .collect();
    
    // 3. Ordenar
    sort_cursos(&mut cursos);
    
    // 4. Devolver JSON
    Ok(json!({ "malla": malla_id, "cursos": cursos }))
}
```

**Complejidad:** O(n) donde n = número de cursos en la malla

#### 2. Obtener Cursos por Semestre

```rust
async fn cursos_por_semestre_handler(
    malla_id: String, 
    semestre: i32
) -> Response {
    // 1. Cargar malla
    let map = load_malla_map(&malla_id, sheet)?;
    
    // 2. Filtrar por semestre
    let mut cursos: Vec<CursoDto> = map
        .values()
        .filter(|r| r.semestre == Some(semestre))
        .map(ramo_to_dto)
        .collect();
    
    // 3. Ordenar
    sort_cursos(&mut cursos);
    
    // 4. Devolver JSON
    Ok(json!({
        "malla": malla_id,
        "semestre": semestre,
        "cursos": cursos
    }))
}
```

**Complejidad:** O(n) donde n = número de cursos en la malla

#### 3. Calcular Cursos Elegibles

```rust
async fn cursos_recomendados_handler(
    malla_id: String,
    ramos_aprobados: Vec<String>
) -> Response {
    // 1. Cargar malla
    let map = load_malla_map(&malla_id, sheet)?;
    
    // 2. Identificar IDs de ramos aprobados
    let aprobados_ids = identificar_aprobados(&map, &ramos_aprobados);
    
    // 3. Filtrar cursos elegibles
    let elegibles = map
        .values()
        .filter(|r| {
            // No está aprobado
            !aprobados_ids.contains(&r.id) &&
            // Todos los prerequisitos están aprobados
            prerequisitos_cumplidos(r, &aprobados_ids)
        })
        .map(ramo_to_dto)
        .collect();
    
    // 4. Ordenar
    sort_cursos(&mut elegibles);
    
    // 5. Devolver JSON
    Ok(json!({
        "malla": malla_id,
        "total_elegibles": elegibles.len(),
        "cursos": elegibles
    }))
}
```

**Complejidad:** O(n) donde n = número de cursos en la malla

### Funciones Auxiliares Clave

#### `load_malla_map()`

```rust
fn load_malla_map(
    malla_id: &str, 
    sheet: Option<String>
) -> Result<HashMap<String, RamoDisponible>, String> {
    // 1. Resolver paths de archivos
    let (malla_path, _oferta_path, porcent_path) = 
        resolve_datafile_paths(malla_id)?;
    
    // 2. Detectar tipo de malla
    let is_mc = malla_path_str.to_lowercase().contains("mc");
    
    // 3. Llamar función optimizada apropiada
    let res = if is_mc {
        leer_mc_con_porcentajes_optimizado(malla_path_str, porcent_path_str)
    } else {
        leer_malla_con_porcentajes_optimizado(malla_path_str, porcent_path_str)
    };
    
    res.map_err(|e| format!("failed to read malla: {}", e))
}
```

**Ventaja:** Usa funciones optimizadas con MapeoMaestro (O(1) lookup vs O(n²) nested loops)

#### `prerequisitos_cumplidos()`

```rust
fn prerequisitos_cumplidos(
    ramo: &RamoDisponible, 
    aprobados_ids: &HashSet<i32>
) -> bool {
    ramo.requisitos_ids
        .iter()
        .all(|req_id| {
            *req_id <= 0 ||  // Sin prerequisito
            aprobados_ids.contains(req_id)  // Prerequisito cumplido
        })
}
```

**Complejidad:** O(k) donde k = número de prerequisitos del curso (típicamente k ≤ 3)

#### `elegibles_desde_malla()`

```rust
fn elegibles_desde_malla(
    map: &HashMap<String, RamoDisponible>,
    aprobados_raw: &[String],
) -> Vec<CursoDto> {
    // 1. Limpiar y normalizar ramos aprobados
    let aprobados_codes_upper: HashSet<String> = 
        aprobados_raw.iter().map(|s| s.to_uppercase()).collect();
    let aprobados_norm: HashSet<String> = 
        aprobados_raw.iter().map(|s| normalize_name(s)).collect();
    
    // 2. Identificar IDs de ramos aprobados
    let mut aprobados_ids: HashSet<i32> = HashSet::new();
    for ramo in map.values() {
        let code_upper = ramo.codigo.to_uppercase();
        let name_norm = normalize_name(&ramo.nombre);
        
        if aprobados_codes_upper.contains(&code_upper) ||
           aprobados_norm.contains(&name_norm) {
            aprobados_ids.insert(ramo.id);
        }
    }
    
    // 3. Filtrar cursos elegibles
    let mut elegibles: Vec<CursoDto> = map
        .values()
        .filter(|r| {
            let code_upper = r.codigo.to_uppercase();
            !aprobados_ids.contains(&r.id) &&
            !aprobados_codes_upper.contains(&code_upper) &&
            prerequisitos_cumplidos(r, &aprobados_ids)
        })
        .map(ramo_to_dto)
        .collect();
    
    // 4. Ordenar
    sort_cursos(&mut elegibles);
    
    elegibles
}
```

**Características:**
- ✅ Acepta códigos o nombres de cursos
- ✅ Normaliza nombres para comparación robusta
- ✅ Maneja mayúsculas/minúsculas y acentos
- ✅ Complejidad O(n) lineal

---


## 🧪 EJEMPLOS DE USO

### Ejemplo 1: Obtener Cursos de Primer Semestre

```bash
curl -X GET "http://localhost:8080/api/mallas/MallaCurricular2020.xlsx/semestres/1/cursos" \
  -H "Accept: application/json"
```

**Respuesta:**
```json
{
  "malla": "MallaCurricular2020.xlsx",
  "semestre": 1,
  "cursos": [
    {
      "id": 1,
      "nombre": "Cálculo I",
      "codigo": "CBM1001",
      "semestre": 1,
      "requisitos_ids": [],
      "electivo": false,
      "dificultad": 0.65,
      "numb_correlativo": 1,
      "critico": true
    }
  ]
}
```

### Ejemplo 2: Calcular Cursos Elegibles para Estudiante Nuevo

```bash
curl -X POST "http://localhost:8080/api/cursos/recomendados" \
  -H "Content-Type: application/json" \
  -d '{
    "malla_id": "MallaCurricular2020.xlsx",
    "ramos_aprobados": []
  }'
```

**Respuesta:** Devuelve todos los cursos de primer semestre (sin prerequisitos)

### Ejemplo 3: Calcular Cursos Elegibles con Historial

```bash
curl -X POST "http://localhost:8080/api/cursos/recomendados" \
  -H "Content-Type: application/json" \
  -d '{
    "malla_id": "MallaCurricular2020.xlsx",
    "ramos_aprobados": [
      "CBM1001",
      "CBM1002",
      "Programación I",
      "CIT2111",
      "Inglés General I"
    ]
  }'
```

**Respuesta:** Devuelve cursos de semestres 2-3 cuyos prerequisitos están cumplidos

### Ejemplo 4: Obtener Todos los Cursos con Hoja Específica

```bash
curl -X GET "http://localhost:8080/api/mallas/MallaCurricular2020.xlsx/cursos?sheet=Malla%202020" \
  -H "Accept: application/json"
```


## 🔗 INTEGRACIÓN CON OTROS ENDPOINTS

### Flujo Completo: Generar Horario

```
1. GET /api/mallas/{id}/cursos
   → Obtener estructura completa de la malla

2. POST /api/cursos/recomendados
   → Calcular cursos elegibles dado historial

3. POST /rutacritica/run
   → Generar horarios óptimos con cursos elegibles
```

### Flujo: Exploración de Malla

```
1. GET /datafiles
   → Listar mallas disponibles

2. GET /datafiles/content?malla=...
   → Ver hojas disponibles en la malla

3. GET /api/mallas/{id}/cursos?sheet=...
   → Obtener cursos de hoja específica
```



