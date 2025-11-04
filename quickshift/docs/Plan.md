# Suposiciones iniciales (usadas para las reglas)

- La malla que subiste contiene: créditos por ramo (esquina sup. derecha), id del ramo (inf. izq.), y dependencias/prerrequisitos (inf. der.). Voy a usar esto como datos base. Duración objetivo: terminar lo antes posible (tope 11 semestres incluyendo titulación).

- Datos adicionales que deben proveerse al correr el algoritmo: porcentaje de reprobación por ramo (dificultad), ranking académico del alumno (percentil o número), tabla de horarios (disponibilidad del alumno y horarios de cada ramo), número de cupos por ramo (si aplica), crédito máximo por semestre (p. ej. 30 o configurable).

- Prerrequisitos son restricciones duras: no puedes programar un ramo sin haber aprobado los prerrequisitos (o simultáneamente si la malla lo permite expresamente).

# Reglas / Principios generales (ordenadas)

## Reglas Obligatorias (sin condiciones de usuario)

**0. — Prerrequisitos obligatorios**: un ramo sólo es elegible si sus prerrequisitos ya están aprobados (o se permiten co-requisitos explícitos).

**1. — Titulación más temprana**: priorizar ramos que acerquen lo antes posible a cumplir requisitos de titulación. Maximizar avance hacia requisitos de titulación (memoria, proyecto, etc.) en los primeros semestres si es posible.

**2. — Probabilidad de aprobación**: ajustar carga académica según tasa de reprobación del ramo (dificultad) y ranking del estudiante.

## Reglas Opcionales del Usuario (filtros configurables)

**3. — Días/horarios libres** (habilitado: sí/no)
- Minimizar ventanas entre clases o dejar días completamente libres.

**4. — Ventana entre actividades** (habilitado: sí/no)
- Espacios mínimos requeridos entre clases consecutivas.

**5. — Preferencias de Profesores** (habilitado: sí/no)
- Priorizar o descartar secciones según docentes preferidos.

**6. — Balance entre líneas de formación** (habilitado: sí/no)
- Mantener proporción equilibrada entre Informática y Telecomunicaciones.

## Reglas Derivadas (antiguas, reordenadas)

**7. — Prioridad por dependencia hacia la titulación**: ramos que desbloquean muchos otros tienen prioridad mayor.

**8. — Minimizar riesgo de retraso**: priorizar ramos críticos antes que optativos.

**9. — Política de reprobación/recuperación**: si un estudiante reprueba, reducir su carga en el siguiente semestre.

**10. — Titulación en semestres finales**: reservar créditos de titulación para semestres finales.


## Puntos Importantes:

Cada archivo excel debe tener su formato estandarizado para que el programa funcione.

- MC = Malla Curricular
- OA = Oferta Académica
- PA = Porcentaje Aprobación

---

## Mapeo JSON de Filtros Opcionales (Request POST)

Cuando el usuario ejecuta `/rutacritica/run`, puede enviar filtros opcionales de la siguiente manera:

```json
{
  "email": "estudiante@example.com",
  "ramos_pasados": ["CBM1000", "CBM1001"],
  "ramos_prioritarios": [],
  "malla": "MiMalla.xlsx",
  
  "filtros": {
    "dias_horarios_libres": {
      "habilitado": false,
      "dias_libres_preferidos": ["VI"],
      "minimizar_ventanas": true
    },
    "ventana_entre_actividades": {
      "habilitado": false,
      "minutos_entre_clases": 15
    },
    "preferencias_profesores": {
      "habilitado": false,
      "profesores_preferidos": ["Dr. García"],
      "profesores_evitar": []
    },
    "balance_lineas": {
      "habilitado": false,
      "lineas": {
        "informatica": 0.6,
        "telecomunicaciones": 0.4
      }
    }
  }
}
```

Cada filtro es completamente opcional. Si `habilitado: false` o no se envía, el sistema ignora ese filtro.
