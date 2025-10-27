# Suposiciones iniciales (usadas para las reglas)

- La malla que subiste contiene: créditos por ramo (esquina sup. derecha), id del ramo (inf. izq.), y dependencias/prerrequisitos (inf. der.). Voy a usar esto como datos base. Duración objetivo: terminar lo antes posible (tope 11 semestres incluyendo titulación).

- Datos adicionales que deben proveerse al correr el algoritmo: porcentaje de reprobación por ramo (dificultad), ranking académico del alumno (percentil o número), tabla de horarios (disponibilidad del alumno y horarios de cada ramo), número de cupos por ramo (si aplica), crédito máximo por semestre (p. ej. 30 o configurable).

- Prerrequisitos son restricciones duras: no puedes programar un ramo sin haber aprobado los prerrequisitos (o simultáneamente si la malla lo permite expresamente).

# Reglas / Principios generales (ordenadas)

0. — Prerrequisitos obligatorios: un ramo sólo es elegible si sus prerrequisitos ya están aprobados (o se permiten co-requisitos explícitos).

1. — Límite de créditos por semestre: no exceder el máximo permitido (configurable). Sugerencia inicial: crédito máximo por semestre = 30; crédito “óptimo” para estudiantes con ranking bajo = 18–22; para ranking alto = 24–30.

2. — Ajuste por dificultad: si un ramo tiene alta tasa de reprobación, reducir la carga total del semestre en X créditos (por ejemplo 3–6 créditos) o no poner más de 1 ramo “difícil” (> umbral) en el mismo semestre.

3. — Prioridad por dependencia hacia la titulación: ramos que desbloquean muchos otros (alto out-degree en grafo de dependencia) tienen prioridad mayor para evitar cuellos de botella.

4. — Acceso por ranking: si un ramo tiene cupos limitados, asignación por ranking; estudiantes con peor ranking deben planificar alternativas o tomarlo en semestres posteriores. El sistema debe ofrecer “planes alternativos” si no obtienen cupo.

5. — Compatibilidad horaria: un ramo sólo es seleccionable si al combinar con los ya seleccionados no hay choque horario. Si hay conflicto, escoger la combinación con mayor probabilidad de pasar (según dificultad + ranking).

6. — Minimizar riesgo de retraso: priorizar ramos críticos para el avance (prerrequisitos de muchos cursos o requisito para titulación) antes que ramos optativos, salvo que la dificultad los haga demasiado arriesgados para el estudiante actual.

7. — Política de reprobación/recuperación: si un estudiante reprueba, en el siguiente semestre reducir su carga un 15–25% y priorizar la reposición del ramo reprobado (si es requisito).

8. — Titulación: reservar los créditos y requisitos de titulación (memoria / proyecto) como semestres finales, evitando sobrecargar el mismo semestre con ramos de alta dificultad más el proceso de titulación.