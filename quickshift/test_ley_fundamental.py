#!/usr/bin/env python3
"""
TEST: LEY FUNDAMENTAL - Validación contra endpoint /solve

Verifica que:
1. Siempre hay ≥1 solución sin filtros
2. NUNCA aparecen cursos aprobados en soluciones
3. Funciona hasta semestre 9

Uso: python3 test_ley_fundamental.py [--server http://localhost:8080]
"""

import json
import requests
import sys
from typing import List, Dict, Tuple

class TestLeyFundamental:
    def __init__(self, server_url: str = "http://127.0.0.1:8080"):
        self.server_url = server_url
        self.endpoint = f"{server_url}/solve"
        self.passed = 0
        self.failed = 0
        self.results = []

    # Cursos por semestre (basado en Malla2020.xlsx)
    CURSOS_POR_SEMESTRE = [
        # Semestre 1
        ["CBM1000", "CBM1001", "CBQ1000", "CIT1000", "FIC1000", "CBM1002"],
        # Semestre 2
        ["CBM1003", "CBF1000", "CIT1010", "CBM1005", "CBM1006", "CBF1001"],
        # Semestre 3
        ["CIT2114", "CIT2107", "CIT1011", "CBF1002", "CIT2007", "CBF1003"],
        # Semestre 4
        ["CIT2204", "CIT2108", "CIT2009", "CBM1007", "CBM1008", "CBF1004"],
        # Semestre 5
        ["CIT2205", "CII1000", "CII1001", "CII1002", "CBF1005", "CBM1009"],
        # Semestre 6
        ["CII1003", "CII1004", "CII1005", "CII1006", "CBF1006", "CBM1010"],
        # Semestre 7
        ["CII1007", "CII1008", "CII1009", "CII1010", "CBF1007", "CBM1011"],
        # Semestre 8
        ["CII1011", "CII1012", "CII1013", "CII1014", "CBF1008", "CBM1012"],
        # Semestre 9
        ["CII1015", "CII1016", "CII1017", "CII1018", "CBF1009", "CBM1013"],
    ]

    def test_ley_fundamental_completa(self) -> bool:
        """Itera por semestres 1-9 aprobando cursos uno por uno"""
        print("\n" + "="*70)
        print("🔬 TEST: LEY FUNDAMENTAL - Iteración por semestres")
        print("="*70)

        ramos_aprobados = []
        contador_total = 0

        for sem_idx, cursos_sem in enumerate(self.CURSOS_POR_SEMESTRE):
            semestre = sem_idx + 1
            print(f"\n📚 SEMESTRE {semestre}")
            print(f"   Cursos disponibles: {len(cursos_sem)}")

            for idx, curso in enumerate(cursos_sem):
                # Agregar el curso a los aprobados
                ramos_aprobados.append(curso)
                contador_total += 1

                print(f"\n   ✓ Aprobado: {curso} ({idx+1}/{len(cursos_sem)})")
                print(f"     Total aprobados: {len(ramos_aprobados)}")

                # Llamar a /solve
                test_passed = self._test_caso_individual(
                    semestre=semestre,
                    ramos_aprobados=ramos_aprobados.copy(),
                    idx_en_semestre=idx + 1
                )

                if test_passed:
                    self.passed += 1
                else:
                    self.failed += 1

        # Resumen
        print("\n" + "="*70)
        print("\n📊 RESUMEN DEL TEST\n")
        print(f"Total de casos: {self.passed + self.failed}")
        print(f"✅ Passed: {self.passed}")
        print(f"❌ Failed: {self.failed}")
        print(f"\n📈 Tasa de éxito: {(self.passed*100)//(self.passed+self.failed)}%")

        # Mostrar fallos
        if self.failed > 0:
            print(f"\n⚠️  FALLOS DETECTADOS:\n")
            for result in self.results:
                if not result["passed"]:
                    print(f"  ❌ {result['test_name']}")
                    print(f"     Razón: {result['reason']}")

        print("\n" + "="*70)
        return self.failed == 0

    def _test_caso_individual(self, semestre: int, ramos_aprobados: List[str], idx_en_semestre: int) -> bool:
        """Ejecuta un caso individual contra /solve"""
        try:
            payload = {
                "email": "test@x.com",
                "malla": "Malla2020.xlsx",
                "sheet": "Malla 2020",
                "ramos_pasados": ramos_aprobados,
                "ramos_prioritarios": [],
                "horarios_preferidos": [],
                "filtros": {}
            }

            response = requests.post(self.endpoint, json=payload, timeout=20)
            response.raise_for_status()
            data = response.json()

            soluciones_count = data.get("soluciones_count", 0)
            soluciones = data.get("soluciones", [])

            # VALIDACIÓN 1: ¿Hay al menos 1 solución?
            if soluciones_count == 0 and len(ramos_aprobados) < len(set(c for sem in self.CURSOS_POR_SEMESTRE for c in sem)):
                test_name = f"Semestre {semestre} - {idx_en_semestre}/6 cursos"
                self.results.append({
                    "test_name": test_name,
                    "passed": False,
                    "reason": f"LEY VIOLADA: 0 soluciones con {len(ramos_aprobados)} cursos aprobados"
                })
                print(f"     ❌ LEY VIOLADA: Sin soluciones")
                return False

            # VALIDACIÓN 2: ¿Hay cursos aprobados en las soluciones?
            ramos_set = set(ramos_aprobados)
            for sol_idx, solucion in enumerate(soluciones):
                codigos_en_sol = [sec["codigo"] for sec in solucion["secciones"]]
                aprobados_en_sol = [c for c in codigos_en_sol if c in ramos_set]

                if aprobados_en_sol:
                    test_name = f"Semestre {semestre} - {idx_en_semestre}/6 (sol {sol_idx+1})"
                    self.results.append({
                        "test_name": test_name,
                        "passed": False,
                        "reason": f"Cursos aprobados en solución: {aprobados_en_sol}"
                    })
                    print(f"     ❌ Cursos aprobados en solución: {aprobados_en_sol}")
                    return False

            # VALIDACIÓN 3: Contar soluciones válidas
            test_name = f"Semestre {semestre} - {idx_en_semestre}/6"
            self.results.append({
                "test_name": test_name,
                "passed": True,
                "reason": f"✅ {soluciones_count} soluciones válidas"
            })
            print(f"     ✅ {soluciones_count} soluciones válidas (sin aprobados)")
            return True

        except Exception as e:
            test_name = f"Semestre {semestre} - {idx_en_semestre}/6"
            self.results.append({
                "test_name": test_name,
                "passed": False,
                "reason": f"ERROR: {str(e)}"
            })
            print(f"     ❌ ERROR: {str(e)}")
            return False

    def test_sin_filtros_garantia(self) -> bool:
        """Verifica que SIN FILTROS siempre hay solución"""
        print("\n" + "="*70)
        print("🔬 TEST: Garantía - Sin filtros = siempre solución")
        print("="*70)

        for sem_idx in range(len(self.CURSOS_POR_SEMESTRE) - 1):  # No el último
            ramos_aprobados = []
            for i in range(sem_idx + 1):
                ramos_aprobados.extend(self.CURSOS_POR_SEMESTRE[i])

            semestre = sem_idx + 1

            try:
                payload = {
                    "email": "test@x.com",
                    "malla": "Malla2020.xlsx",
                    "sheet": "Malla 2020",
                    "ramos_pasados": ramos_aprobados,
                    "ramos_prioritarios": [],
                    "horarios_preferidos": [],
                    "filtros": {}
                }

                response = requests.post(self.endpoint, json=payload, timeout=20)
                data = response.json()
                soluciones_count = data.get("soluciones_count", 0)

                if soluciones_count > 0:
                    print(f"✅ Semestre {semestre+1}: {soluciones_count} soluciones (LEY cumplida)")
                    self.passed += 1
                else:
                    print(f"❌ Semestre {semestre+1}: 0 soluciones (LEY VIOLADA)")
                    self.failed += 1

            except Exception as e:
                print(f"❌ Semestre {semestre+1}: ERROR - {str(e)}")
                self.failed += 1

        print(f"\n✅ Passed: {self.passed} | ❌ Failed: {self.failed}")
        return self.failed == 0

    def run_all_tests(self) -> bool:
        """Ejecuta todos los tests"""
        print("\n🚀 Iniciando validación de LEY FUNDAMENTAL\n")
        print(f"   Servidor: {self.server_url}")
        print(f"   Endpoint: {self.endpoint}\n")

        # Test 1: Iteración completa
        result1 = self.test_ley_fundamental_completa()

        # Test 2: Garantía de soluciones
        result2 = self.test_sin_filtros_garantia()

        print("\n" + "="*70)
        if result1 and result2:
            print("✅ TODOS LOS TESTS PASARON - LEY FUNDAMENTAL VERIFICADA")
        else:
            print("❌ ALGUNOS TESTS FALLARON - REVISAR LOGS")
        print("="*70 + "\n")

        return result1 and result2


if __name__ == "__main__":
    server_url = "http://127.0.0.1:8080"

    # Parsear argumentos
    if len(sys.argv) > 2 and sys.argv[1] == "--server":
        server_url = sys.argv[2]

    tester = TestLeyFundamental(server_url=server_url)
    success = tester.run_all_tests()

    sys.exit(0 if success else 1)
