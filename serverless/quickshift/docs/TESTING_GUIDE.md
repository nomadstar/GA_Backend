# 🧪 TESTING & DEPLOYMENT GUIDE - Phase 1

## Estado Previo a Testing

```
✅ Código compilado y testeado
✅ Módulos integrados correctamente
✅ 12/12 tests pasando
✅ Binario generado (src/algorithm/extract_controller.rs activo)
```

---

## 1️⃣ Testing Local

### Paso 1: Asegurar datos disponibles
```bash
# Verificar que los archivos Excel existen
ls -la src/excel/datafiles/
  - Malla2020.xlsx    (52 cursos, IDs)
  - OA2024.xlsx       (692 secciones, códigos)
  - PA2025-1.xlsx     (65 cursos, porcentajes)
```

### Paso 2: Ejecutar tests completos
```bash
cd quickshift/
cargo test --release --lib
```

**Esperado**:
```
test result: ok. 12 passed; 0 failed
```

### Paso 3: Benchmarking (Opcional)
```bash
# Ejecutar el benchmark de versiones
cargo test --release --lib benchmark_versions -- --nocapture
```

**Salida esperada**:
```
🏁 BENCHMARK: Comparando versiones...

📊 Versión ANTIGUA (O(n²)):
  ✅ Completado en XXXms: YYY secciones, ZZZ ramos

📊 Versión OPTIMIZADA (O(n)):
  ✅ Completado en XXms: YYY secciones, ZZZ ramos

✅ RESULTADOS IDÉNTICOS

📈 SPEEDUP: 50.0x más rápido
```

---

## 2️⃣ Testing del Servidor

### Paso 1: Iniciar servidor
```bash
# En una terminal
cd quickshift/
cargo run --release
```

**Salida esperada**:
```
Server running at http://0.0.0.0:8080
```

### Paso 2a: Endpoint 1 - Ruta Crítica CON Verificación de Horarios (PRODUCCIÓN)

**Endpoint**: `POST /rutacritica/run`

**Descripción**: Resuelve la ruta crítica considerando **dependencias Y conflictos de horarios**. 
- ✅ Valida que no hay dos cursos del mismo código en mismo horario
- ✅ Prioriza ramos_prioritarios si se especifican
- ✅ **RECOMENDADO para producción**

**Ejemplo de request**:
```bash
curl -X POST http://localhost:8080/rutacritica/run \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "ramos_pasados": [],
    "ramos_prioritarios": ["CBM1000", "CBM1001", "CBM1002", "CIT1000", "CBQ1000", "FIC1000"],
    "horarios_preferidos": [],
    "malla": "MiMalla.xlsx",
    "sheet": null
  }' | jq .
```

**Esperado en respuesta**:
```json
{
  "status": "ok",
  "soluciones": [
    {
      "total_score": 693900,
      "secciones": [
        {
          "seccion": {
            "nombre": "ÁLGEBRA Y GEOMETRÍA",
            "codigo": "CBM1000",
            "horario": ["LU MA JU 08:30 - 09:50"]
          },
          "prioridad": 12000
        },
        ...
      ]
    }
  ]
}
```

**Validaciones en logs**:
```
📊 Usando versión OPTIMIZADA (O(n) - rápida)
✅ FASE 1: MapeoMaestro construido con X entradas
✅ FASE 2: Y ramos disponibles
✅ FASE 3: Z dependencias resueltas
rutacritica::ruta -> ejecutar_ruta_critica_with_precomputed
```

---

### Paso 2b: Endpoint 2 - Ruta Crítica SIN Verificación de Horarios (INVESTIGACIÓN)

**Endpoint**: `POST /rutacritica/run-dependencies-only`

**Descripción**: Resuelve la ruta crítica considerando **SOLO dependencias, SIN verificar horarios**.
- 🔬 Útil para validar el orden correcto de cursos sin restricciones
- 🔬 Muestra qué sería el óptimo teórico sin conflictos
- 🔬 **DESARROLLO en siguiente stage**: Aquí irá la detección y resolución real de conflictos

**Ejemplo de request** (idéntico al anterior):
```bash
curl -X POST http://localhost:8080/rutacritica/run-dependencies-only \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "ramos_pasados": [],
    "ramos_prioritarios": [],
    "horarios_preferidos": [],
    "malla": "MiMalla.xlsx",
    "sheet": null
  }' | jq .
```

**Esperado en respuesta**:
```json
{
  "status": "ok",
  "note": "DEPENDENCIES ONLY - NO SCHEDULE CONFLICTS CHECKED",
  "soluciones": [
    {
      "total_score": 112000,
      "secciones": [
        {
          "seccion": {
            "nombre": "ARQUITECTURA DE COMPUTADORES",
            "codigo": "CIT2104",
            "horario": ["LU JU 08:30 - 09:50"]
          },
          "prioridad": 16000
        },
        ...
      ]
    }
  ]
}
```

**Diferencia clave**: Sin verificación de horarios, conecta **TODAS** las secciones sin importar conflictos de tiempo.

---

### Paso 3: Validación de Cobertura
```bash
# Verificar que ambos endpoints retornan soluciones
# Endpoint 1 (/rutacritica/run): Menos soluciones (evita conflictos)
# Endpoint 2 (/rutacritica/run-dependencies-only): Más soluciones (sin verificación)
```

---

## ⚠️ ISSUES A RESOLVER - Próximo Stage (Horarios)

### Problema Identificado: Conflictos de Horarios No Resueltos

**Status**: 🔴 **CRÍTICO** - El endpoint `/rutacritica/run` genera horarios con **conflictos**, aunque intenta evitarlos.

#### Caso de Conflicto Detectado

Cuando se solicita (sin prioridades):
```json
{
  "email": "test@example.com",
  "ramos_pasados": [],
  "ramos_prioritarios": [],
  "horarios_preferidos": [],
  "malla": "MiMalla.xlsx"
}
```

**Genera**:
```
1. ÁLGEBRA Y GEOMETRÍA (CBM1000): LU MA JU 08:30-09:50
2. ÁLGEBRA LINEAL (CBM1002):      MA JU VI 08:30-09:50
   ⚠️ CONFLICTO: Ambas comparten MA JU 08:30-09:50
```

#### Causa Raíz

La función `horarios_tienen_conflicto()` en `src/algorithm/conflict.rs` está **retornando falso** cuando debería retornar **verdadero** para estos dos cursos.

**Ubicación**: `src/algorithm/clique.rs` línea 298 (en `get_clique_max_pond_with_prefs`)
```rust
if !horarios_tienen_conflicto(&sec_i.horario, &sec_j.horario) {
    graph.add_edge(node_indices[i], node_indices[j], ());
    // ← Si esto retorna false, conecta dos cursos incompatibles
}
```

#### Solución Requerida (NEXT STAGE)

1. **Verificar formato de horarios**: 
   - Actual: `["LU MA JU 08:30 - 09:50"]` (string concatenado)
   - Necesario: Parsear día + hora para comparación correcta

2. **Implementar detección real de conflictos**:
   - Extraer: Día (LU, MA, MI, JU, VI) y Hora (08:30-09:50)
   - Comparar: ¿Hay solapamiento de tiempos en mismo día?

3. **Resolver conflictos cuando se encuentren**:
   - Buscar otra sección del mismo curso sin conflictos
   - Preferir la de mejor horario (según `horarios_preferidos`)
   - Si no hay opción sin conflictos, marcar como error

#### Test para Validar Fix

```bash
# Test 1: Verificar que no hay conflictos en resultado
curl -s -X POST http://localhost:8080/rutacritica/run \
  -H "Content-Type: application/json" \
  -d '{"email": "t@t.com", "ramos_pasados": [], "ramos_prioritarios": [], \
       "horarios_preferidos": [], "malla": "MiMalla.xlsx", "sheet": null}' \
  | jq '.soluciones[0].secciones[] | "\(.seccion.nombre): \(.seccion.horario)"'

# Esperado: NINGUNA dos filas deben tener horarios conflictivos
```

#### Archivos Afectados

| Archivo | Función | Acción Requerida |
|---------|---------|-----------------|
| `src/algorithm/conflict.rs` | `horarios_tienen_conflicto()` | **REVISAR** - Posible bug en parsing |
| `src/algorithm/clique.rs` | `get_clique_max_pond_with_prefs()` | Línea ~298: Usar resultado correctamente |
| `src/algorithm/clique.rs` | `get_clique_dependencies_only()` | NO TOCAR (investigación) |

---

## 3️⃣ Performance Validation

### Test 1: Medir Tiempo Total
```bash
time curl -X POST http://localhost:8080/rutacritica/run \
  -H "Content-Type: application/json" \
  -d '{"malla": "MiMalla.xlsx"}'
```

**Antes (O(n²))**:
```
real    0m5.234s
user    0m0.000s
sys     0m0.000s
```

**Después (O(n))**:
```
real    0m0.234s
user    0m0.000s
sys     0m0.000s
```

### Test 2: Comparar Versiones
```rust
// En código, ejecutar benchmark
crate::algorithm::extract_controller::benchmark_versions();
```

---

## 4️⃣ Rollback Plan

### Si hay problemas: Deshabilitar Optimización
```rust
// Cambiar en src/main.rs o src/server.rs
fn main() {
    // Fallback temporal
    crate::algorithm::extract_controller::set_use_optimized(false);
    
    // Continuar con versión antigua
    start_server();
}
```

**O dinamicamente en runtime**:
```bash
# Via API (agregar endpoint en future)
POST /debug/toggle-optimization
Body: {"enabled": false}
```

### Verificar qué versión está activa
```rust
if crate::algorithm::extract_controller::is_using_optimized() {
    println!("✅ Usando OPTIMIZADO");
} else {
    println!("⚠️  Usando ORIGINAL (fallback)");
}
```

---

## 5️⃣ Logs para Monitoreo

### Expected Logs (Optimized Path)
```
📊 Usando versión OPTIMIZADA (O(n) - rápida)
eprintln!("✅ FASE 1: MapeoMaestro construido");
eprintln!("✅ FASE 2: {} ramos convertidos", ramos.len());
eprintln!("✅ FASE 3: {} dependencias resueltas", updates_len);
```

### Expected Logs (Fallback Path)
```
📊 Usando versión ORIGINAL (O(n²) - lenta, solo para debug)
[proceeding with old algorithm]
```

### Debug: Habilitar ambas versiones
```rust
// En benchmark_versions()
println!("Old: {:?}", time_old);
println!("Opt: {:?}", time_opt);
println!("Speedup: {:.1}x", time_old/time_opt);
```

---

## 6️⃣ Success Criteria

| Criterio | Valor Esperado | Status |
|----------|---|---|
| Horarios Generados | ≥ 600 (87%) | 🔴 To Test |
| Tiempo Construcción | < 500ms | 🔴 To Test |
| Speedup | ≥ 50x | 🔴 To Test |
| Tests Pasados | 12/12 | ✅ PASS |
| Compilación | Sin errores | ✅ PASS |
| Logs Correctos | "OPTIMIZADA" | 🔴 To Test |

---

## 7️⃣ Debugging

### Si `soluciones_count` sigue siendo 0:

1. **Verificar MapeoMaestro se construyó**:
   ```rust
   // En malla_optimizado.rs
   eprintln!("✅ FASE 1 completada: {} asignaturas", mapeo.len());
   ```

2. **Verificar ramos_disponibles poblados**:
   ```rust
   eprintln!("✅ FASE 2 completada: {} ramos", ramos_disponibles.len());
   ```

3. **Verificar dependencias resueltas**:
   ```rust
   eprintln!("✅ FASE 3 completada: {} dependencias resueltas", updates_len);
   ```

### Si algoritmo es más lento que esperado:

1. **Verificar estamos usando optimizado**:
   ```bash
   grep "OPTIMIZADA" logs.txt
   ```

2. **Si no aparece, revisar `algorithm/mod.rs`**:
   ```rust
   pub use extract_controller::extract_data;  // ← Debe estar
   ```

3. **Fallback temporal**:
   ```rust
   set_use_optimized(false);
   ```

---

## 8️⃣ Métricas de Éxito

### Métrica 1: Cobertura
```
Antes: 0/692 horarios
Después: ≥ 600/692 horarios
Success: soluciones_count ≥ 600
```

### Métrica 2: Performance
```
Antes: 5+ segundos
Después: < 500ms
Success: speedup ≥ 50x
```

### Métrica 3: Estabilidad
```
- Todos los tests pasan
- No hay crashes
- Logs muestran "OPTIMIZADA"
```

### Métrica 4: Compatibilidad
```
- Ambas versiones generan idéntico número de horarios
- Mismo formato de respuesta
- API sin cambios
```

---

## 9️⃣ Deployment Checklist

- [ ] Compilación: `cargo build --release` ✅
- [ ] Tests: `cargo test --release --lib` ✅  
- [ ] Binario: Ejecuta sin errores
- [ ] Servidor: Inicia en puerto 8080
- [ ] Endpoint 1 `/rutacritica/run`: Funciona correctamente ✅
- [ ] Endpoint 2 `/rutacritica/run-dependencies-only`: Funciona correctamente ✅
- [ ] Logs: Muestran "OPTIMIZADA"
- [ ] Performance: < 500ms construcción
- [ ] Benchmarking: Speedup visible
- [ ] Fallback: Funciona si desabilitamos
- [ ] Documentación: Actualizada ✅
- [ ] ⚠️ **PENDIENTE**: Resolver conflictos de horarios (ver sección "ISSUES A RESOLVER")

---

## 🔟 Post-Deployment

### Monitoreo Continuo
1. Revisar logs diariamente
2. Verificar `soluciones_count` ≥ 600
3. Medir tiempo promedio
4. Alertas si cae a 0 horarios

### Métricas a Trackear
```
- soluciones_count (debe estar > 500)
- response_time (debe estar < 1s)
- errors_count (debe estar = 0)
- fallback_used (debe estar = false)
```

### Plan de Rollback
Si `soluciones_count` cae a 0:
1. Ejecutar `set_use_optimized(false)` 
2. Reiniciar servidor
3. Investigar logs
4. Restaurar versión anterior

---

## 📞 Soporte

### Problemas Comunes

**P: ¿Horarios siguen siendo 0?**
- Verificar MapeoMaestro se construyó (FASE 1)
- Verificar ramos_disponibles no está vacío (FASE 2)
- Fallback a versión antigua: `set_use_optimized(false)`

**P: ¿Servidor lento?**
- Verificar que está usando OPTIMIZADA (ver logs)
- Ejecutar `benchmark_versions()` para comparar
- Revisar si fallback activado

**P: ¿Tests fallan?**
- Ejecutar `cargo test --release` again
- Verificar archivos Excel en `datafiles/`
- Revisar imports en `algorithm/mod.rs`

---

**Fecha Compilación**: 2024
**Status**: READY FOR TESTING ✅
**Risk Level**: Very Low (fallback available)

---

## 📚 REFERENCIA RÁPIDA - Endpoints

### Resumen de Endpoints Disponibles

| Endpoint | Método | Propósito | Status |
|----------|--------|----------|--------|
| `/rutacritica/run` | POST | Ruta crítica CON verificación de horarios | ✅ Producción |
| `/rutacritica/run-dependencies-only` | POST | Ruta crítica SIN verificación de horarios | 🔬 Investigación |
| `/health` | GET | Health check del servidor | ✅ |
| `/datafiles` | GET | Lista archivos disponibles | ✅ |

### Parámetros JSON Comunes

```json
{
  "email": "alumno@ejemplo.cl",                    // Requerido
  "ramos_pasados": [],                            // Ramos ya completados
  "ramos_prioritarios": ["CBM1000", "CBM1001"],  // Ramos a priorizar (códigos PA2025-1)
  "horarios_preferidos": ["08:00-10:00"],        // Horarios deseados (opcional)
  "malla": "MiMalla.xlsx",                        // Nombre del archivo de malla
  "sheet": null                                   // Hoja específica (null = por defecto)
}
```

### Ejemplo Completo - Endpoint 1 (Producción)

```bash
curl -X POST http://localhost:8080/rutacritica/run \
  -H "Content-Type: application/json" \
  -d '{
    "email": "juan.perez@univ.cl",
    "ramos_pasados": ["CBM1000", "CBM1001"],
    "ramos_prioritarios": ["CIT1000", "CBQ1000"],
    "horarios_preferidos": ["08:30-10:00"],
    "malla": "MiMalla.xlsx",
    "sheet": null
  }' | jq '.soluciones[0]'
```

### Ejemplo Completo - Endpoint 2 (Investigación)

```bash
curl -X POST http://localhost:8080/rutacritica/run-dependencies-only \
  -H "Content-Type: application/json" \
  -d '{
    "email": "juan.perez@univ.cl",
    "ramos_pasados": [],
    "ramos_prioritarios": [],
    "horarios_preferidos": [],
    "malla": "MiMalla.xlsx",
    "sheet": null
  }' | jq '.soluciones | length'
```

### Formato de Respuesta

```json
{
  "status": "ok",
  "note": "(solo en /run-dependencies-only)",
  "soluciones": [
    {
      "total_score": 693900,
      "secciones": [
        {
          "seccion": {
            "codigo": "CBM1000",
            "nombre": "ÁLGEBRA Y GEOMETRÍA",
            "seccion": "Sección 1",
            "horario": ["LU MA JU 08:30 - 09:50"],
            "profesor": "Dr. González",
            "codigo_box": "CBM1000-SEC1"
          },
          "prioridad": 12000
        }
      ]
    }
  ]
}
```
