# 📋 ROADMAP: Próximos Pasos para Mapeo Maestro

## Fase 1: Integración en malla.rs (1-2 horas)

### 1.1 Reemplazar búsquedas nested
- [ ] Modificar `leer_malla_con_porcentajes()` en `malla.rs`
- [ ] Usar `MapeoMaestro::get()` en lugar de HashMap anidados
- [ ] Eliminar loops nested que hacen O(n²)

### 1.2 Simplificar lógica de NO-ELECTIVOS
```rust
// ANTES (problemático):
for (norm_name, data) in porcent_by_name.iter() {
    for (oa_norm, cod) in oa_nombre_to_codigo.iter() {
        // O(n²) - dos loops anidados
    }
}

// DESPUÉS (simple):
let mapeo = construir_mapeo_maestro(...)?;
for (nombre, _) in malla.iter() {
    if let Some(asig) = mapeo.get(&nombre) {
        // O(1) - acceso directo
    }
}
```

### 1.3 Simplificar lógica de ELECTIVOS
- [ ] Los electivos de malla (IDs 44,46,50,51,52) ya están asignados a códigos únicos
- [ ] Solo verificar que `mapeo.get_by_codigo_pa(codigo)` existe
- [ ] Si no existe, usar fallback (código ID)

### 1.4 Testing
- [ ] Verificar que no hay cuelgues en el servidor
- [ ] Ejecutar POST /rutacritica/run y capturar logs
- [ ] Validar que genera schedules (no vacío)

## Fase 2: SQL Persistence (2-3 horas)

### 2.1 Diseño de tabla
```sql
CREATE TABLE asignaturas (
    nombre_normalizado VARCHAR(255) PRIMARY KEY,
    nombre_real VARCHAR(255) NOT NULL,
    id_malla INT UNIQUE,
    codigo_oa2024 VARCHAR(20) UNIQUE,
    codigo_pa2025 VARCHAR(20) UNIQUE,
    porcentaje_aprobacion DECIMAL(5,2),
    es_electivo BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Índices
CREATE INDEX idx_codigo_oa ON asignaturas(codigo_oa2024);
CREATE INDEX idx_codigo_pa ON asignaturas(codigo_pa2025);
CREATE INDEX idx_id_malla ON asignaturas(id_malla);
CREATE INDEX idx_es_electivo ON asignaturas(es_electivo);
```

### 2.2 Implementar DB loader
- [ ] Nueva función: `cargar_mapeo_desde_db()`
- [ ] Conectar a PostgreSQL/SQLite
- [ ] Cache en memoria al iniciar servidor
- [ ] Tiempo de carga < 1 segundo

### 2.3 Sincronización Excel → DB
- [ ] Script: `sync_excel_to_db.rs`
- [ ] Ejecutable: `cargo run --bin sync_excel_to_db`
- [ ] Detección automática de cambios
- [ ] Logging de cambios (auditoría)

### 2.4 API para gestión de mapeo
- [ ] `GET /mapeo/asignaturas` - listar todas
- [ ] `GET /mapeo/buscar?nombre=...` - buscar por nombre
- [ ] `POST /mapeo/asignaturas` - agregar
- [ ] `PUT /mapeo/asignaturas/{nombre_norm}` - actualizar
- [ ] `DELETE /mapeo/asignaturas/{nombre_norm}` - eliminar

## Fase 3: Multi-año Support (1-2 horas)

### 3.1 Extender esquema DB
```sql
-- Tabla de años académicos
CREATE TABLE anos_academicos (
    id INT PRIMARY KEY,
    ano INT UNIQUE,
    periodo INT,
    activo BOOLEAN DEFAULT FALSE
);

-- Extender tabla asignaturas
ALTER TABLE asignaturas ADD COLUMN ano_academico INT;
ALTER TABLE asignaturas ADD COLUMN carrera VARCHAR(100);

-- Nueva PK
ALTER TABLE asignaturas 
    DROP CONSTRAINT asignaturas_pkey;
ALTER TABLE asignaturas 
    ADD PRIMARY KEY (nombre_normalizado, ano_academico, carrera);
```

### 3.2 Generalizador de Mapeo
- [ ] `MapeoMaestro::for_year(year)` - filtrar por año
- [ ] `MapeoMaestro::for_carrera(carrera)` - filtrar por carrera
- [ ] Histórico de cambios: qué cambió entre años

### 3.3 Herramienta de análisis de cambios
- [ ] Python script: `analizar_cambios_anos.py`
- [ ] Salida: "En 2025 vs 2024: X códigos cambiaron"
- [ ] Reporte de códigos descontinuados

## Fase 4: Testing & Validation (1 hora)

### 4.1 Unit tests
```rust
#[test]
fn test_mapeo_construccion() { }

#[test]
fn test_mapeo_busqueda_exacta() { }

#[test]
fn test_mapeo_busqueda_por_codigo_oa() { }

#[test]
fn test_electivos_sin_repeticion() { }

#[test]
fn test_cobertura_85_porciento() { }
```

### 4.2 Integration tests
- [ ] Leer 3 archivos Excel
- [ ] Construir mapeo
- [ ] Generar schedule completo
- [ ] Validar que no hay cuelgues

### 4.3 Performance tests
- [ ] Tiempo de construcción < 1s
- [ ] Búsqueda promedio < 1ms
- [ ] Memoria < 100MB

## Fase 5: Documentation (30 min)

### 5.1 Actualizar docs
- [ ] README.md con nueva arquitectura
- [ ] API docs con nuevos endpoints
- [ ] Architecture decision record (ADR)

### 5.2 Guía de operación
- [ ] Cómo sincronizar Excel → DB
- [ ] Cómo agregar nuevo año académico
- [ ] Cómo manejar cambios de códigos

## Fase 6: Deployment (30 min)

### 6.1 Preparar migración
- [ ] Backup de datos actuales
- [ ] Script de migración (si hay DB antiguo)
- [ ] Plan de rollback

### 6.2 Documentar cambios
- [ ] CHANGELOG.md
- [ ] Version bump (semver)
- [ ] Release notes

---

## Estimación Total
- Fase 1: 1-2 horas ✅ (puede hacerse hoy)
- Fase 2: 2-3 horas
- Fase 3: 1-2 horas
- Fase 4: 1 hora
- Fase 5: 30 min
- Fase 6: 30 min
- **Total: 6-9 horas** (1-2 días de trabajo concentrado)

## Prioridad Recomendada
1. **Fase 1 (CRÍTICA)**: Eliminar cuelgues, hacer funcionar el sistema
2. **Fase 4 (IMPORTANTE)**: Validar que funciona
3. **Fase 2 (IMPORTANTE)**: Persistencia para escalabilidad
4. **Fase 3 (ÚTIL)**: Multi-año support
5. **Fase 5-6 (MANTENIMIENTO)**: Documentación y deployment

---

## Dependencies/Prerrequisitos

### Para Fase 1
- [ ] Rust toolchain (ya instalado ✅)
- [ ] Conocimiento de malla.rs (tenemos 📚)

### Para Fase 2
- [ ] PostgreSQL o SQLite
- [ ] sqlx o diesel crate
- [ ] Conexión a DB configurada

### Para Fase 3
- [ ] Multi-tenant support library (opcional)
- [ ] Query builder flexible

---

## Notas de Implementación

### Evitar
- ❌ Búsquedas nested O(n²)
- ❌ Archivos Excel como "DB" (persistencia débil)
- ❌ Código duplicado para cada año
- ❌ Hard-coded paths

### Preferir
- ✅ HashMap/índices para búsqueda rápida
- ✅ SQL con índices
- ✅ Parámetros configurables
- ✅ DRY (Don't Repeat Yourself)
- ✅ Logging y auditoría

---

## Métricas de Éxito

| Métrica | Antes | Después | Target |
|---------|-------|---------|--------|
| **Secciones encontradas** | 0/692 | TBD | ≥600 |
| **Tiempo construcción** | 5+ seg (cuelgue) | <1 seg | <500ms |
| **Complejidad búsqueda** | O(n²) | O(1) | O(1) |
| **Cobertura efectiva** | 0% | TBD | ≥85% |
| **Schedules generados** | 0 | TBD | ≥80% |

---

## Contacto & Preguntas

Si tienes dudas en cualquier fase:
1. Revisar `MAPEO_MAESTRO.md` para arquitectura
2. Revisar `RESUMEN_SOLUCION.md` para contexto
3. Ejecutar `verify_mapeo.py` para validar datos
4. Leer tests para entender uso esperado

---

**Última actualización**: 30 de octubre de 2025
**Autor**: GitHub Copilot + tu insight sobre códigos
**Status**: Ready to implement Phase 1 ✅
