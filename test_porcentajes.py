#!/usr/bin/env python3
"""
Test para validar que los porcentajes/dificultad se cargan correctamente
"""

import json
import requests
import time

# Esperar a que el servidor esté listo
time.sleep(1)

# Test request
test_data = {
    "email": "test@test.cl",
    "malla": "MiMalla.xlsx",
    "ramos_pasados": [],
    "ramos_prioritarios": [],
    "horarios_preferidos": [],
    "bloques_descuentos": {}
}

print("📊 Test de carga de porcentajes...")
print("=" * 60)

try:
    response = requests.post(
        "http://127.0.0.1:8080/solve",
        json=test_data,
        timeout=10
    )
    
    result = response.json()
    print(f"\n✅ Respuesta recibida:")
    print(json.dumps(result, indent=2, ensure_ascii=False))
    
    if response.status_code == 200:
        print(f"\n✅ Status: 200 OK")
        print(f"   Documentos leídos: {result.get('documentos_leidos')}")
        print(f"   Soluciones encontradas: {result.get('soluciones_count')}")
    else:
        print(f"\n❌ Status: {response.status_code}")
        
except Exception as e:
    print(f"\n❌ Error: {e}")
    
print("\n📌 Nota: Los porcentajes se cargan internamente.")
print("   Revisa los logs del servidor (grep 'PA:') para confirmar.")
