# 📊 ANÁLISIS: PROBLEMA DE CÓDIGOS Y SOLUCIÓN CON MAPEO MAESTRO

## El Problema Descubierto

La universidad cambió los **códigos de asignaturas entre 2024 y 2025**, pero mantuvieron los **nombres** (aproximadamente) iguales.

### Ejemplo del Engaño:
```
INGLÉS GENERAL II:
  - En OA2024 (Jan 2024):  CIG1002
  - En PA2025-1 (Jan 2025): CIG1013
  
El MISMO curso tiene CÓDIGOS DIFERENTES según el año

CRIPTOGRAFÍA Y SEGURIDAD EN REDES:
  - En OA2024: CIT2105
  - En PA2025-1: CIT2113
```

### Los 3 Sistemas de Códigos:
1. **Malla2020.xlsx**: Usa IDs numéricos (1-57) + Nombres
2. **OA2024.xlsx**: Usa códigos alfanuméricos (CBF1000, CIT2109) + Nombres + Secciones/Horarios
3. **PA2025-1.xlsx**: Usa códigos alfanuméricos DIFERENTES (CBM1001, CIT2013) + Nombres + Porcentajes

### Coincidencias:
- ✅ **40/59** códigos de OA2024 coinciden con PA2025-1
- ❌ **19** códigos solo en OA2024 (no hay oferta en jan 2025)
- ❌ **25** códigos solo en PA2025-1 (no hay secciones en 2024)

## La Solución: Mapeo Maestro por NOMBRE NORMALIZADO

**Clave Universal: NOMBRE NORMALIZADO** (minúsculas, sin acentos, espacios limpios)

```
NOMBRE NORMALIZADO = "criptografia y seguridad en redes"
  ├─ ID Malla:         ❓ (puede no existir si es electivo)
  ├─ Código OA2024:    "CIT2105"
  ├─ Código PA2025-1:  "CIT2113" (DIFERENTE)
  ├─ Porcentaje:       100%
  └─ Es Electivo:      true
```

### Ventajas:
1. **Único identificador**: Cada asignatura = 1 nombre normalizado
2. **Resistente a cambios**: Códigos pueden cambiar, nombre no (casi nunca)
3. **Deduplicación automática**: Si el nombre es igual, los datos se fusionan
4. **Búsqueda eficiente**: O(1) por nombre, O(n) para búsquedas secundarias (si es necesario)

## Estructura de Datos Implementada

### `MapeoAsignatura` (src/excel/mapeo.rs)
```rust
pub struct MapeoAsignatura {
    pub nombre_normalizado: String,      // Clave única
    pub nombre_real: String,             // "Criptografía y Seguridad en Redes"
    pub id_malla: Option<i32>,           // ID de Malla2020 (si existe)
    pub codigo_oa2024: Option<String>,   // "CIT2105"
    pub codigo_pa2025: Option<String>,   // "CIT2113"
    pub porcentaje_aprobacion: Option<f64>, // 100.0
    pub es_electivo: bool,               // true/false
}
```

### `MapeoMaestro` (src/excel/mapeo.rs)
```rust
pub struct MapeoMaestro {
    asignaturas: HashMap<String, MapeoAsignatura>,
}
```

### Constructor: `construir_mapeo_maestro()` (src/excel/mapeo_builder.rs)
```
Paso 1: Leer PA2025-1 (fuente de verdad: códigos y porcentajes)
  └─ Crea: nombre_norm → MapeoAsignatura

Paso 2: Leer OA2024 (agrega código_oa2024 a asignaturas existentes)
  └─ Si existe por nombre, actualiza; si no, crea nueva

Paso 3: Leer Malla2020 (agrega id_malla)
  └─ Si existe por nombre, actualiza; si no, ignora (es auxiliar)

Resultado: Mapeo unificado con todos los datos
```

## Flujo de Uso (Futuro)

### Antes (Problemático):
```
Malla2020 (nombre: "Cálculo II")
  ↓ (búsqueda por nombre)
OA2024 (código: CBM1003, nombre: "CÁLCULO II")
  ↓ (búsqueda por código en PA2025-1) ← FALLA: código cambió a CBM1003 en 2025
PA2025-1 (código: CBM1003, porcentaje: 53.13%)
  ✅ Pero solo funciona si el código no cambió
```

### Después (Robusto):
```
Malla2020 (nombre: "Cálculo II")
  ↓ (normalizar nombre)
"calculo ii"
  ↓ (buscar en MapeoMaestro)
MapeoAsignatura {
  nombre_normalizado: "calculo ii",
  codigo_oa2024: "CBM1003",       ← De OA2024
  codigo_pa2025: "CBM1003",       ← De PA2025-1 (puede ser diferente)
  porcentaje_aprobacion: 53.13,   ← De PA2025-1
  id_malla: 8,                    ← De Malla2020
  es_electivo: false
}
✅ Funciona incluso si los códigos cambian, porque usa nombre como llave
```

## Próximos Pasos

### 1. ✅ HECHO: Estructuras de datos (`mapeo.rs`)
   - Definidas `MapeoAsignatura` y `MapeoMaestro`
   - Métodos de búsqueda: por nombre, código_oa, código_pa, id_malla

### 2. ✅ HECHO: Constructor (`mapeo_builder.rs`)
   - Lee los 3 archivos Excel
   - Construye mapeo unificado
   - Manejo de duplicados/fusión

### 3. ⏳ TODO: Integrar en `malla.rs`
   - Simplificar lógica de búsqueda
   - Reemplazar búsquedas nested con consultas al MapeoMaestro
   - Eliminar ciclos O(n²)

### 4. ⏳ TODO: SQL para Persistencia
   - Tabla `asignaturas` con columnas: nombre_norm, nombre, id_malla, codigo_oa, codigo_pa, porcentaje, es_electivo
   - Índices en: nombre_norm (PK), codigo_oa, codigo_pa, id_malla
   - Cache en memoria al inicio
   - Sincronización con archivos Excel periódicamente

## Beneficios del Enfoque

| Aspecto | Antes | Después |
|---------|-------|---------|
| **Clave** | Código (cambia año a año) | Nombre (estable) |
| **Búsqueda** | O(n²) múltiples ficheros | O(1) MapeoMaestro |
| **Deduplicación** | Manual, error-prone | Automática por nombre |
| **Mantenimiento** | Modificar 3 archivos | 1 tabla SQL + 1 cache |
| **Cambios códigos** | ❌ Quiebra | ✅ Automáticamente tolerado |

## Archivos Creados

- ✅ `src/excel/mapeo.rs` - Estructuras de datos
- ✅ `src/excel/mapeo_builder.rs` - Constructor desde Excel
- ✅ `src/excel/mod.rs` - Módulo exportado

## Estado Actual de Compilación

✅ **Compila sin errores** (5.32s)
⚠️  Advertencias de código no usado (funciones legacy)
✅ Listos para integrar en lógica principal

## Testing Recomendado

```rust
#[test]
fn test_mapeo_fusion():
    // Verificar que mismo nombre de cursos con códigos diferentes
    // se fusionan en un solo MapeoAsignatura

#[test]
fn test_cambio_codigos():
    // Simular cambio de código entre años
    // Verificar que búsqueda funciona por nombre

#[test]
fn test_electivos_unicos():
    // Verificar que cada electivo tiene nombre único
    // (aunque tenga código diferente en 2024 vs 2025)
```

---

**Propuesta de Migración a SQL**:

Cuando estés listo, podemos crear una tabla SQL que replique esta estructura:

```sql
CREATE TABLE asignaturas (
    nombre_normalizado VARCHAR(255) PRIMARY KEY,
    nombre_real VARCHAR(255) NOT NULL,
    id_malla INT UNIQUE,
    codigo_oa2024 VARCHAR(20) UNIQUE,
    codigo_pa2025 VARCHAR(20) UNIQUE,
    porcentaje_aprobacion DECIMAL(5,2),
    es_electivo BOOLEAN,
    CREATED_AT TIMESTAMP DEFAULT NOW(),
    UPDATED_AT TIMESTAMP DEFAULT NOW()
);
```

Así tenemos **persistencia** y podemos escalar a múltiples años, carreras, etc.
