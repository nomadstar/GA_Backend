#!/usr/bin/env python3
"""
Normalizar MC2020.xlsx - corregir nombres y eliminar duplicados
"""

import pandas as pd
from pathlib import Path

DATAFILES = Path("src/datafiles")

# Mapeo de correcciones de nombres
CORRECTIONS = {
    "CIT1010": "PROGRAMACIÓN",  # La primera aparición es correcta
    "CBM1006": "CÁLCULO II",
    "CII2100": "INTRODUCCIÓN A LA ECONOMÍA",  # Normalizar espacios
    "CIT3325": "INTELIGENCIA ARTIFICIAL",
    "CIT2009": "BASES DE DATOS",
    "CIT2207": "EVALUACIÓN DE PROYECTOS TIC",
    "CIT3203": "PROYECTO EN TICS I",
    "CIT5002": "PRÁCTICA PROFESIONAL 1",
    "CIG1003": "INGLÉS GENERAL I",
}

def normalize_mc2020():
    print("📋 Normalizando MC2020.xlsx...")
    
    # Leer archivo
    df = pd.read_excel(DATAFILES / "MC2020.xlsx")
    
    print(f"\n📊 Estado inicial:")
    print(f"   Filas totales: {len(df)}")
    print(f"   Códigos únicos: {df['Código'].nunique()}")
    
    # Aplicar correcciones de nombres
    print(f"\n🔧 Aplicando correcciones:")
    for code, correct_name in CORRECTIONS.items():
        mask = df['Código'] == code
        if mask.any():
            print(f"   {code}: '{correct_name}'")
            df.loc[mask, 'Nombre Asignatura'] = correct_name
    
    # Eliminar duplicados (mantener primera ocurrencia)
    duplicates_before = len(df)
    df = df.drop_duplicates(subset=['Código'], keep='first')
    duplicates_removed = duplicates_before - len(df)
    
    if duplicates_removed > 0:
        print(f"\n❌ Duplicados eliminados: {duplicates_removed}")
    
    # Guardar
    output = DATAFILES / "MC2020_normalizado.xlsx"
    df.to_excel(output, index=False)
    
    print(f"\n✅ Normalización completada")
    print(f"   Filas después: {len(df)}")
    print(f"   Códigos únicos: {df['Código'].nunique()}")
    print(f"   📁 Guardado en: {output}")
    
    # Mostrar lista de cursos
    print(f"\n📚 Cursos normalizados ({len(df)}):")
    for idx, row in df.iterrows():
        print(f"   {row['Código']}: {row['Nombre Asignatura']}")

if __name__ == "__main__":
    normalize_mc2020()

if __name__ == "__main__":
    normalize_mc2020()
