#!/usr/bin/env python3
"""
Normalizar OA20251.xlsx - corregir nombres para que coincidan con MC2020_normalizado
"""

import pandas as pd
from pathlib import Path

DATAFILES = Path("src/datafiles")

# Mapeo de correcciones de nombres (DEBEN COINCIDIR con MC2020_normalizado)
CORRECTIONS = {
    "CIT1010": "PROGRAMACIÓN",
    "CBM1006": "CÁLCULO II",
    "CII2100": "INTRODUCCIÓN A LA ECONOMÍA",
    "CIT3325": "INTELIGENCIA ARTIFICIAL",
    "CIT2009": "BASES DE DATOS",
    "CIT2207": "EVALUACIÓN DE PROYECTOS TIC",
    "CIT3203": "PROYECTO EN TICS I",
    "CIT5002": "PRÁCTICA PROFESIONAL 1",
    "CIG1003": "INGLÉS GENERAL I",
}

def normalize_oa20251():
    print("📋 Normalizando OA20251.xlsx...")
    
    # Leer archivo
    df = pd.read_excel(DATAFILES / "OA20251.xlsx")
    
    print(f"\n📊 Estado inicial:")
    print(f"   Filas totales: {len(df)}")
    print(f"   Códigos únicos: {df['Asignatura'].nunique()}")
    
    # Aplicar correcciones de nombres
    print(f"\n🔧 Aplicando correcciones:")
    for code, correct_name in CORRECTIONS.items():
        mask = df['Asignatura'] == code
        if mask.any():
            count = mask.sum()
            print(f"   {code}: '{correct_name}' ({count} filas)")
            df.loc[mask, 'Nombre Asig.'] = correct_name
    
    # Guardar
    output = DATAFILES / "OA20251_normalizado.xlsx"
    df.to_excel(output, index=False)
    
    print(f"\n✅ Normalización completada")
    print(f"   Filas totales: {len(df)}")
    print(f"   Códigos únicos: {df['Asignatura'].nunique()}")
    print(f"   📁 Guardado en: {output}")
    
    # Verificar cobertura con MC2020_normalizado
    print(f"\n🔍 Comparando con MC2020_normalizado.xlsx...")
    mc_df = pd.read_excel(DATAFILES / "MC2020_normalizado.xlsx")
    mc_codes = set(mc_df['Código'].dropna().unique())
    oa_codes = set(df['Asignatura'].dropna().unique())
    
    only_in_mc = mc_codes - oa_codes
    only_in_oa = oa_codes - mc_codes
    
    print(f"\n   MC2020: {len(mc_codes)} cursos únicos")
    print(f"   OA20251: {len(oa_codes)} cursos únicos")
    print(f"   Overlap: {len(mc_codes & oa_codes)} cursos")
    
    if only_in_mc:
        print(f"\n   ⚠️  En MC2020 pero NO en OA20251 ({len(only_in_mc)}):")
        for code in sorted(only_in_mc):
            print(f"      - {code}")
    
    if only_in_oa:
        print(f"\n   ℹ️  En OA20251 pero NO en MC2020 ({len(only_in_oa)}):")
        for code in sorted(list(only_in_oa)[:10]):
            print(f"      - {code}")
        if len(only_in_oa) > 10:
            print(f"      ... y {len(only_in_oa) - 10} más")

if __name__ == "__main__":
    normalize_oa20251()
