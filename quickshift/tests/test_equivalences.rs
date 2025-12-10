use quickshift::excel::{cargar_equivalencias, aplicar_equivalencias};
use std::collections::HashMap;

#[test]
fn test_cargar_equivalencias() {
    // Usar la malla MC2020moded que tiene equivalencias
    let malla_path = "src/datafiles/MC2020moded.xlsx";
    
    match cargar_equivalencias(malla_path) {
        Ok(equivalencias) => {
            eprintln!("✅ {} equivalencias cargadas", equivalencias.len());
            
            // Verificar que CIG1014 está mapeado a CIG1003
            if let Some(mapped) = equivalencias.get("CIG1014") {
                eprintln!("✅ CIG1014 mapped to {}", mapped);
                assert_eq!(mapped, "CIG1003");
            } else {
                eprintln!("⚠️  CIG1014 no encontrado en equivalencias");
            }
            
            // Verificar que CIG1013 está mapeado a CIG1003
            if let Some(mapped) = equivalencias.get("CIG1013") {
                eprintln!("✅ CIG1013 mapped to {}", mapped);
                assert_eq!(mapped, "CIG1003");
            }
        }
        Err(e) => {
            eprintln!("❌ Error cargando equivalencias: {}", e);
            panic!("Falló cargar_equivalencias: {}", e);
        }
    }
}

#[test]
fn test_aplicar_equivalencias() {
    let mut codigos = vec![
        "CIG1014".to_string(),
        "CIT2100".to_string(),
        "CIG1013".to_string(),
    ];
    
    let mut equivalencias = HashMap::new();
    equivalencias.insert("CIG1014".to_string(), "CIG1003".to_string());
    equivalencias.insert("CIG1013".to_string(), "CIG1003".to_string());
    
    let resultado = aplicar_equivalencias(&codigos, &equivalencias);
    
    eprintln!("Original: {:?}", codigos);
    eprintln!("Resultado: {:?}", resultado);
    
    // CIG1014 debe convertirse a CIG1003
    assert!(resultado.contains(&"CIG1003".to_string()));
    // CIT2100 debe permanecer igual (sin equivalencia)
    assert!(resultado.contains(&"CIT2100".to_string()));
    
    // Contar instancias de CIG1003 (debería ser 2: una de CIG1014 y una de CIG1013)
    let count_cig1003 = resultado.iter().filter(|c| c == &"CIG1003").count();
    assert_eq!(count_cig1003, 2, "Debería haber 2 instancias de CIG1003");
}
