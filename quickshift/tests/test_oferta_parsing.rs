use quickshift::excel::leer_oferta_academica_excel;

#[test]
fn test_load_oa20251() {
    eprintln!("\n🧪 TEST: Cargar OA20251.xlsx y parsear secciones");
    
    match leer_oferta_academica_excel("OA20251.xlsx") {
        Ok(secciones) => {
            eprintln!("✅ Cargadas {} secciones desde OA20251.xlsx", secciones.len());
            if secciones.len() > 0 {
                eprintln!("\n📋 Primeras 5 secciones:");
                for sec in secciones.iter().take(5) {
                    eprintln!("  - Código: {}, Nombre: {}, Sección: {}, Horarios: {:?}", 
                        sec.codigo, sec.nombre, sec.seccion, sec.horario);
                }
            } else {
                eprintln!("⚠️  No se cargaron secciones!");
            }
        }
        Err(e) => {
            eprintln!("❌ Error al cargar OA20251.xlsx: {}", e);
            panic!("No se pudo cargar OA20251.xlsx");
        }
    }
}
