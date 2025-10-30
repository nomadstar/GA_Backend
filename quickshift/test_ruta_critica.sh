#!/bin/bash

# 🧪 TEST SCRIPT: Validar que el sistema genera Ruta Crítica
# Este script hace curl con datos simulados para verificar que Phase 1 funciona

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  🧪 TEST RUTA CRÍTICA - Phase 1 Validation                  ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo

# Colores para output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 1. Verificar que el servidor está corriendo
echo -e "${BLUE}1️⃣  Verificando que el servidor está disponible...${NC}"
if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Servidor no responde en :8080${NC}"
    echo "   Inicia el servidor con: cd quickshift && cargo run --release"
    exit 1
fi
echo -e "${GREEN}✅ Servidor disponible${NC}"
echo

# 2. Preparar JSON con datos simulados
echo -e "${BLUE}2️⃣  Preparando request con datos simulados...${NC}"

# JSON con ramos reales que debería encontrar en OA2024.xlsx
REQUEST_JSON='{
  "email": "test@example.com",
  "ramos_pasados": [
    "CIG1001",
    "CIT1001"
  ],
  "ramos_prioritarios": [
    "CIG1002",
    "CIT2104"
  ],
  "horarios_preferidos": [
    "08:00-10:00",
    "10:00-12:00"
  ],
  "malla": "MiMalla.xlsx",
  "sheet": null
}'

echo -e "${GREEN}✅ JSON preparado${NC}"
echo

# 3. Hacer el request
echo -e "${BLUE}3️⃣  Enviando POST /rutacritica/run...${NC}"
echo "   URL: http://localhost:8080/rutacritica/run"
echo

RESPONSE=$(curl -s -X POST http://localhost:8080/rutacritica/run \
  -H "Content-Type: application/json" \
  -d "$REQUEST_JSON")

echo -e "${YELLOW}Response recibido:${NC}"
echo "$RESPONSE" | jq . 2>/dev/null || echo "$RESPONSE"
echo

# 4. Analizar respuesta
echo -e "${BLUE}4️⃣  Analizando resultados...${NC}"

# Extraer métricas clave
SOLUCIONES_COUNT=$(echo "$RESPONSE" | jq -r '.soluciones_count // 0')
DOCUMENTOS=$(echo "$RESPONSE" | jq -r '.documentos_leidos // 0')
ERROR=$(echo "$RESPONSE" | jq -r '.error // ""')

echo

# 5. Validaciones
if [ ! -z "$ERROR" ]; then
    echo -e "${RED}❌ ERROR EN LA RESPUESTA:${NC}"
    echo "   $ERROR"
    echo
    exit 1
fi

echo "   Documentos leídos: $DOCUMENTOS"
echo "   Soluciones (Rutas Críticas) generadas: $SOLUCIONES_COUNT"
echo

# 6. Validar métricas de éxito
echo -e "${BLUE}5️⃣  Validando éxito de Phase 1...${NC}"
echo

SUCCESS=true

# Antes de Phase 1: soluciones_count = 0
# Después de Phase 1: soluciones_count >= 600

if [ "$SOLUCIONES_COUNT" -eq 0 ]; then
    echo -e "${RED}❌ FALLA: soluciones_count = 0${NC}"
    echo "   Problema: El sistema sigue sin generar horarios"
    echo "   Verificar:"
    echo "   - MapeoMaestro se construyó correctamente"
    echo "   - Nombres se normalizaron correctamente"
    echo "   - Códigos coinciden entre archivos"
    SUCCESS=false
elif [ "$SOLUCIONES_COUNT" -lt 600 ]; then
    echo -e "${YELLOW}⚠️  ADVERTENCIA: soluciones_count = $SOLUCIONES_COUNT${NC}"
    echo "   Esperado: >= 600 (87% de cobertura)"
    echo "   Se generaron algunos horarios, pero menos de lo esperado"
else
    echo -e "${GREEN}✅ ÉXITO: $SOLUCIONES_COUNT soluciones generadas${NC}"
    echo "   Esto es lo esperado (600+ horarios)"
fi

echo

# 7. Mostrar primera solución como muestra
echo -e "${BLUE}6️⃣  Mostrando primera solución como muestra...${NC}"
FIRST_SOLUCION=$(echo "$RESPONSE" | jq '.soluciones[0] // {}')
if [ ! -z "$FIRST_SOLUCION" ] && [ "$FIRST_SOLUCION" != "{}" ]; then
    echo "   Score total: $(echo "$FIRST_SOLUCION" | jq -r '.total_score')"
    SECCIONES_COUNT=$(echo "$FIRST_SOLUCION" | jq '.secciones | length')
    echo "   Secciones en esta solución: $SECCIONES_COUNT"
    echo "   Primera sección:"
    echo "$FIRST_SOLUCION" | jq '.secciones[0]' | head -10
else
    echo "   (No hay soluciones para mostrar)"
fi

echo
echo "╔════════════════════════════════════════════════════════════════╗"
if [ "$SUCCESS" = true ] && [ "$SOLUCIONES_COUNT" -ge 600 ]; then
    echo -e "║  ${GREEN}✅ PHASE 1 VALIDATION SUCCESSFUL${NC}                         ║"
    echo -e "║  ${GREEN}Ruta Crítica se genera correctamente${NC}                      ║"
else
    echo -e "║  ${RED}⚠️  PHASE 1 NEEDS INVESTIGATION${NC}                           ║"
fi
echo "╚════════════════════════════════════════════════════════════════╝"
