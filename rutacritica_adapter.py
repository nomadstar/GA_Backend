
import sys
import os
import json
from pathlib import Path

# Cambiar al directorio de RutaCritica
os.chdir(r"/home/ignatus/Documents/GitHub/GA_Backend/RutaCritica")
sys.path.insert(0, r"/home/ignatus/Documents/GitHub/GA_Backend/RutaCritica")

# Suprimir la ejecución automática al importar
import numpy as np
import pandas as pd
import networkx as nx

# Cargar el módulo manualmente sin ejecutar el código al final
with open("rutaCritica.py", "r", encoding="utf-8") as f:
    code = f.read()
    
# Encontrar donde está la llamada getRamoCritico('MiMalla.xlsx')
# y removerla temporalmente para poder importar
code_lines = code.split("\n")
exec_globals = {"__name__": "__main__", "np": np, "pd": pd, "nx": nx}

# Ejecutar solo las funciones, no la llamada final
for i, line in enumerate(code_lines):
    if line.strip().startswith("getRamoCritico('MiMalla.xlsx')"):
        # No ejecutar esta línea
        continue
    exec("\n".join(code_lines[:i+1]), exec_globals)

# Ahora getRamoCritico está disponible
getRamoCritico = exec_globals["getRamoCritico"]

# Ejecutar
try:
    ramos_disponibles, malla_name = getRamoCritico(r"/home/ignatus/Documents/GitHub/GA_Backend/RutaCritica/MiMalla.xlsx")
    
    result = {
        "success": True,
        "ramos_count": len(ramos_disponibles),
        "ramos_list": sorted(list(ramos_disponibles.keys())),
        "malla": malla_name,
        "ramos_detail": {
            k: {
                "codigo": v["codigo"],
                "nombre": v["nombre"],
                "holgura": v["holgura"],
                "critico": v["critico"],
                "numb_correlativo": v["numb_correlativo"]
            }
            for k, v in ramos_disponibles.items()
        }
    }
    
    print(json.dumps(result, ensure_ascii=False))
    
except Exception as e:
    result = {
        "success": False,
        "error": str(e),
        "error_type": type(e).__name__
    }
    print(json.dumps(result, ensure_ascii=False))
