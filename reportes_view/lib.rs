#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(unexpected_cfgs)]

use ink::prelude::vec::Vec;
use ink::prelude::string::String;

// Tipos compartidos
// (Orden y derives exactos)

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum RolUsuario {
    Comprador,
    Vendedor,
    Ambos,
}

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum EstadoOrden {
    Pendiente,
    Enviada,
    Recibida,
    Cancelada,
}

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Usuario {
    pub direccion: AccountId,
    pub rol: RolUsuario,
    pub reputacion_como_comprador: u32,
    pub reputacion_como_vendedor: u32,
}

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Producto {
    pub id: u32,
    pub nombre: String,
    pub descripcion: String,
    pub precio: u128, // Balance
    pub cantidad: u32,
    pub categoria: String,
    pub vendedor: AccountId,
}

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Orden {
    pub id: u32,
    pub comprador: AccountId,
    pub vendedor: AccountId,
    pub producto_id: u32,
    pub cantidad: u32,
    pub estado: EstadoOrden,
    pub comprador_califico: bool,
    pub vendedor_califico: bool,
    pub comprador_solicita_cancelacion: bool,
    pub vendedor_acepta_cancelacion: bool,
}

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ReporteProductoVendido {
    pub nombre_producto: String,
    pub total_vendido: u32,
}

pub type AccountId = ink::primitives::AccountId;
pub type Balance = u128;

// -------------------------------------------------------------------------
// INTERFAZ DEL CONTRATO REMOTO
// -------------------------------------------------------------------------
#[ink::trait_definition]
pub trait MarketplaceTrait {
    #[ink(message)]
    fn obtener_todos_los_productos(&self) -> Vec<Producto>;
    
    #[ink(message)]
    fn obtener_todas_las_ordenes(&self) -> Vec<Orden>;
    
    #[ink(message)]
    fn obtener_todos_los_usuarios(&self) -> Vec<Usuario>;
}

// -------------------------------------------------------------------------
// CONTRATO REPORTES
// -------------------------------------------------------------------------
#[ink::contract]
mod reportes_view {
    use super::*;

    #[ink(storage)]
    pub struct ReportesView {
        marketplace_contract: AccountId,
    }

    impl ReportesView {
        /// Constructor principal del contrato de reportes.
        ///
        /// Inicializa el contrato almacenando la dirección del contrato principal
        /// de Marketplace para poder realizar las consultas cross-contract.
        ///
        /// # Parámetros
        /// * `direccion_marketplace` - AccountId del contrato `marketplace_principal` desplegado.
        #[ink(constructor)]
        pub fn nuevo(direccion_marketplace: AccountId) -> Self {
            Self {
                marketplace_contract: direccion_marketplace,
            }
        }

        /// Retorna los 5 vendedores con mayor reputación.
        ///
        /// Filtra los usuarios que tienen rol `Vendedor` o `Ambos`, los ordena
        /// por su `reputacion_como_vendedor` de mayor a menor y toma los primeros 5.
        ///
        /// # Retorno
        /// * `Vec<Usuario>`: Lista de hasta 5 usuarios.
        #[ink(message)]
        pub fn top_5_vendedores(&self) -> Vec<Usuario> {
            let usuarios = self.obtener_usuarios_para_reportes();
            Self::filtrar_top_5_vendedores(usuarios)
        }

        fn filtrar_top_5_vendedores(mut usuarios: Vec<Usuario>) -> Vec<Usuario> {
            usuarios.sort_by(|a, b| b.reputacion_como_vendedor.cmp(&a.reputacion_como_vendedor));
            usuarios.into_iter()
                .filter(|u| matches!(u.rol, RolUsuario::Vendedor | RolUsuario::Ambos))
                .take(5)
                .collect()
        }

        /// Retorna los 5 compradores con mayor reputación.
        ///
        /// Filtra los usuarios que tienen rol `Comprador` o `Ambos`, los ordena
        /// por su `reputacion_como_comprador` de mayor a menor y toma los primeros 5.
        ///
        /// # Retorno
        /// * `Vec<Usuario>`: Lista de hasta 5 usuarios.
        #[ink(message)]
        pub fn top_5_compradores(&self) -> Vec<Usuario> {
            let usuarios = self.obtener_usuarios_para_reportes();
            Self::filtrar_top_5_compradores(usuarios)
        }

        fn filtrar_top_5_compradores(mut usuarios: Vec<Usuario>) -> Vec<Usuario> {
            usuarios.sort_by(|a, b| b.reputacion_como_comprador.cmp(&a.reputacion_como_comprador));
            usuarios.into_iter()
                .filter(|u| matches!(u.rol, RolUsuario::Comprador | RolUsuario::Ambos))
                .take(5)
                .collect()
        }

        /// Calcula los productos más vendidos (basado en órdenes recibidas).
        ///
        /// Itera sobre todas las órdenes con estado `Recibida`, suma las cantidades
        /// por producto y devuelve el top 5 ordenado por volumen total de ventas.
        ///
        /// # Retorno
        /// * `Vec<ReporteProductoVendido>`: Lista de estructuras con nombre y total vendido.
        #[ink(message)]
        pub fn productos_mas_vendidos(&self) -> Vec<ReporteProductoVendido> {
            let (productos, ordenes) = self.obtener_datos_productos();
            Self::calcular_productos_mas_vendidos(productos, ordenes)
        }

        fn calcular_productos_mas_vendidos(productos: Vec<Producto>, ordenes: Vec<Orden>) -> Vec<ReporteProductoVendido> {
            let mut reporte_map: ink::prelude::collections::BTreeMap<u32, (String, u32)> = ink::prelude::collections::BTreeMap::new();

            for orden in ordenes {
                if matches!(orden.estado, EstadoOrden::Recibida) {
                    for producto in &productos {
                        if producto.id == orden.producto_id {
                            reporte_map.entry(producto.id)
                                .and_modify(|(_, cant)| *cant = cant.saturating_add(orden.cantidad))
                                .or_insert((producto.nombre.clone(), orden.cantidad));
                            break;
                        }
                    }
                }
            }

            let mut reportes: Vec<ReporteProductoVendido> = reporte_map.into_iter()
                .map(|(_, (nombre, cantidad))| ReporteProductoVendido {
                    nombre_producto: nombre,
                    total_vendido: cantidad,
                })
                .collect();

            reportes.sort_by(|a, b| b.total_vendido.cmp(&a.total_vendido));
            reportes.truncate(5);
            reportes
        }

        /// Calcula la cantidad total de órdenes donde participa un usuario.
        ///
        /// Cuenta tanto las órdenes donde el usuario es comprador como donde es vendedor.
        ///
        /// # Parámetros
        /// * `usuario` - AccountId del usuario a consultar.
        ///
        /// # Retorno
        /// * `u32`: Cantidad total de órdenes asociadas.
        #[ink(message)]
        #[allow(clippy::cast_possible_truncation)]
        pub fn obtener_total_ordenes_usuario(&self, usuario: AccountId) -> u32 {
            let ordenes = self.obtener_ordenes();
            Self::calcular_ordenes_usuario(ordenes, usuario)
        }

        fn calcular_ordenes_usuario(ordenes: Vec<Orden>, usuario: AccountId) -> u32 {
            ordenes.iter()
                .filter(|o| o.comprador == usuario || o.vendedor == usuario)
                .count() as u32
        }

        /// Retorna la dirección del contrato marketplace asociado.
        ///
        /// Útil para verificar que el contrato de reportes está apuntando al
        /// marketplace correcto.
        #[ink(message)]
        pub fn obtener_marketplace(&self) -> AccountId {
            self.marketplace_contract
        }

        // =====================================================================
        // FUNCIONES DE ACCESO A DATOS (MOCKABLES)
        // =====================================================================

        /// Usuarios (Producción).
        #[cfg(not(test))]
        fn obtener_usuarios_para_reportes(&self) -> Vec<Usuario> {
            let marketplace: ink::contract_ref!(MarketplaceTrait) = self.marketplace_contract.into();
            marketplace.obtener_todos_los_usuarios()
        }

        /// Usuarios (Mock).
        #[cfg(test)]
        fn obtener_usuarios_para_reportes(&self) -> Vec<Usuario> {
            // Data mock
            let u1 = Usuario {
                direccion: AccountId::from([0x90; 32]),
                rol: RolUsuario::Vendedor,
                reputacion_como_comprador: 0,
                reputacion_como_vendedor: 100,
            };
            let u2 = Usuario {
                direccion: AccountId::from([0x91; 32]),
                rol: RolUsuario::Comprador,
                reputacion_como_comprador: 50,
                reputacion_como_vendedor: 0,
            };
            vec![u1, u2]
        }

        /// Productos y datos (Producción).
        #[cfg(not(test))]
        fn obtener_datos_productos(&self) -> (Vec<Producto>, Vec<Orden>) {
            let marketplace: ink::contract_ref!(MarketplaceTrait) = self.marketplace_contract.into();
            (marketplace.obtener_todos_los_productos(), marketplace.obtener_todas_las_ordenes())
        }

        /// Productos y datos (Mock).
        #[cfg(test)]
        fn obtener_datos_productos(&self) -> (Vec<Producto>, Vec<Orden>) {
            let p1 = Producto {
                id: 1,
                nombre: "Producto Mock".into(),
                descripcion: "Desc".into(),
                precio: 100,
                cantidad: 10,
                categoria: "Cat".into(),
                vendedor: AccountId::from([0x90; 32]),
            };
            let o1 = Orden {
                id: 1,
                comprador: AccountId::from([0x91; 32]),
                vendedor: AccountId::from([0x90; 32]),
                producto_id: 1,
                cantidad: 5,
                estado: EstadoOrden::Recibida,
                comprador_califico: false,
                vendedor_califico: false,
                comprador_solicita_cancelacion: false,
                vendedor_acepta_cancelacion: false,
            };
            (vec![p1], vec![o1])
        }

        /// Órdenes  (Producción).
        #[cfg(not(test))]
        fn obtener_ordenes(&self) -> Vec<Orden> {
            let marketplace: ink::contract_ref!(MarketplaceTrait) = self.marketplace_contract.into();
            marketplace.obtener_todas_las_ordenes()
        }

        /// Órdenes (Mock).
        #[cfg(test)]
        fn obtener_ordenes(&self) -> Vec<Orden> {
             let o1 = Orden {
                id: 1,
                comprador: AccountId::from([0x91; 32]),
                vendedor: AccountId::from([0x90; 32]),
                producto_id: 1,
                cantidad: 1,
                estado: EstadoOrden::Pendiente,
                comprador_califico: false,
                vendedor_califico: false,
                comprador_solicita_cancelacion: false,
                vendedor_acepta_cancelacion: false,
            };
            vec![o1]
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // =====================================================================
        // TESTS ESTRUCTURALES BÁSICOS
        // =====================================================================

        #[test]
        fn test_crear_reportes_view() {
            let marketplace_id = AccountId::from([0x01; 32]);
            let reportes = ReportesView::nuevo(marketplace_id);
            
            assert_eq!(reportes.obtener_marketplace(), marketplace_id);
        }

        #[test]
        fn test_obtener_marketplace() {
            let marketplace_id = AccountId::from([0x02; 32]);
            let reportes = ReportesView::nuevo(marketplace_id);
            
            assert_eq!(reportes.obtener_marketplace(), marketplace_id);
        }

        #[test]
        fn test_estructura_usuario() {
            let usuario = Usuario {
                direccion: AccountId::from([0x03; 32]),
                rol: RolUsuario::Vendedor,
                reputacion_como_comprador: 5,
                reputacion_como_vendedor: 10,
            };

            assert_eq!(usuario.reputacion_como_vendedor, 10);
            assert_eq!(usuario.reputacion_como_comprador, 5);
        }

        #[test]
        fn test_estructura_producto() {
            let producto = Producto {
                id: 1,
                nombre: "Producto Test".into(),
                descripcion: "Descripción test".into(),
                precio: 1000,
                cantidad: 5,
                categoria: "Electrónica".into(),
                vendedor: AccountId::from([0x04; 32]),
            };

            assert_eq!(producto.id, 1);
            assert_eq!(producto.cantidad, 5);
            assert_eq!(producto.precio, 1000);
        }

        #[test]
        fn test_estructura_orden() {
            let orden = Orden {
                id: 1,
                comprador: AccountId::from([0x05; 32]),
                vendedor: AccountId::from([0x06; 32]),
                producto_id: 1,
                cantidad: 2,
                estado: EstadoOrden::Recibida,
                comprador_califico: true,
                vendedor_califico: false,
                comprador_solicita_cancelacion: false,
                vendedor_acepta_cancelacion: false,
            };

            assert_eq!(orden.id, 1);
            assert_eq!(orden.cantidad, 2);
            assert_eq!(orden.estado, EstadoOrden::Recibida);
            assert!(orden.comprador_califico);
        }

        #[test]
        fn test_estructura_reporte_producto_vendido() {
            let reporte = ReporteProductoVendido {
                nombre_producto: "Laptop".into(),
                total_vendido: 15,
            };

            assert_eq!(reporte.nombre_producto, "Laptop");
            assert_eq!(reporte.total_vendido, 15);
        }

        #[test]
        fn test_rol_usuario_enum() {
            let rol1 = RolUsuario::Comprador;
            let rol2 = RolUsuario::Vendedor;
            let rol3 = RolUsuario::Ambos;

            assert_eq!(rol1, RolUsuario::Comprador);
            assert_eq!(rol2, RolUsuario::Vendedor);
            assert_eq!(rol3, RolUsuario::Ambos);
        }

        #[test]
        fn test_estado_orden_enum() {
            let estado1 = EstadoOrden::Pendiente;
            let estado2 = EstadoOrden::Enviada;
            let estado3 = EstadoOrden::Recibida;
            let estado4 = EstadoOrden::Cancelada;

            assert_eq!(estado1, EstadoOrden::Pendiente);
            assert_eq!(estado2, EstadoOrden::Enviada);
            assert_eq!(estado3, EstadoOrden::Recibida);
            assert_eq!(estado4, EstadoOrden::Cancelada);
        }

        // =====================================================================
        // TESTS PARA COBERTURA DE LÍNEAS (Datos simulados)
        // =====================================================================

        fn crear_producto_test(id: u32, nombre: &str, vendedor: AccountId) -> Producto {
            Producto {
                id,
                nombre: nombre.into(),
                descripcion: "Descripción test".into(),
                precio: 1000,
                cantidad: 10,
                categoria: "Test".into(),
                vendedor,
            }
        }

        fn crear_usuario_test(dir: AccountId, rol: RolUsuario, rep_comprador: u32, rep_vendedor: u32) -> Usuario {
            Usuario {
                direccion: dir,
                rol,
                reputacion_como_comprador: rep_comprador,
                reputacion_como_vendedor: rep_vendedor,
            }
        }

        fn crear_orden_test(
            id: u32,
            comprador: AccountId,
            vendedor: AccountId,
            producto_id: u32,
            cantidad: u32,
            estado: EstadoOrden,
        ) -> Orden {
            Orden {
                id,
                comprador,
                vendedor,
                producto_id,
                cantidad,
                estado,
                comprador_califico: false,
                vendedor_califico: false,
                comprador_solicita_cancelacion: false,
                vendedor_acepta_cancelacion: false,
            }
        }

        // =====================================================================
        // TESTS PARA top_5_vendedores - Cobertura de lógica de ordenamiento y filtrado
        // =====================================================================

        #[test]
        fn test_top_5_vendedores_crea_vector() {
            let resultado = ReportesView::filtrar_top_5_vendedores(Vec::new());
            assert_eq!(resultado.len(), 0);
        }

        #[test]
        fn test_top_5_vendedores_filtra_por_rol() {
            // Verifica que se filtra correctamente por rol Vendedor/Ambos
            let usuario_vendedor = crear_usuario_test(
                AccountId::from([0x10; 32]),
                RolUsuario::Vendedor,
                10,
                50,
            );
            let usuario_comprador = crear_usuario_test(
                AccountId::from([0x11; 32]),
                RolUsuario::Comprador,
                50,
                10,
            );
            let usuario_ambos = crear_usuario_test(
                AccountId::from([0x12; 32]),
                RolUsuario::Ambos,
                30,
                30,
            );

            let usuarios = vec![usuario_vendedor, usuario_comprador, usuario_ambos];
            let top_5 = ReportesView::filtrar_top_5_vendedores(usuarios);

            assert_eq!(top_5.len(), 2);
            assert!(top_5.iter().any(|u| u.rol == RolUsuario::Vendedor));
            assert!(top_5.iter().any(|u| u.rol == RolUsuario::Ambos));
            assert!(!top_5.iter().any(|u| u.rol == RolUsuario::Comprador));
        }

        #[test]
        fn test_top_5_vendedores_ordena_por_reputacion() {
            // Verifica que el sorting por reputación funcionaría correctamente
            let usuarios = vec![
                crear_usuario_test(AccountId::from([0x20; 32]), RolUsuario::Vendedor, 0, 10),
                crear_usuario_test(AccountId::from([0x21; 32]), RolUsuario::Vendedor, 0, 50),
                crear_usuario_test(AccountId::from([0x22; 32]), RolUsuario::Vendedor, 0, 30),
            ];

            let top_5 = ReportesView::filtrar_top_5_vendedores(usuarios);

            assert_eq!(top_5[0].reputacion_como_vendedor, 50);
            assert_eq!(top_5[1].reputacion_como_vendedor, 30);
            assert_eq!(top_5[2].reputacion_como_vendedor, 10);
        }

        #[test]
        fn test_top_5_vendedores_respeta_limite_5() {
            // Verifica que solo retorna máximo 5 vendedores
            let usuarios: Vec<Usuario> = (0..10)
                .map(|i| crear_usuario_test(
                    AccountId::from([i as u8; 32]),
                    RolUsuario::Vendedor,
                    0,
                    100 - i as u32 * 5,
                ))
                .collect();



            let top_5 = ReportesView::filtrar_top_5_vendedores(usuarios);

            assert_eq!(top_5.len(), 5);
            assert_eq!(top_5[0].reputacion_como_vendedor, 100);
            assert_eq!(top_5[4].reputacion_como_vendedor, 80);
        }

        // =====================================================================
        // TESTS PARA top_5_compradores - Similar a vendedores
        // =====================================================================

        #[test]
        fn test_top_5_compradores_filtra_por_rol() {
            let usuario_comprador = crear_usuario_test(
                AccountId::from([0x30; 32]),
                RolUsuario::Comprador,
                50,
                10,
            );
            let usuario_vendedor = crear_usuario_test(
                AccountId::from([0x31; 32]),
                RolUsuario::Vendedor,
                10,
                50,
            );

            let usuarios = vec![usuario_comprador, usuario_vendedor];
            let top_1 = ReportesView::filtrar_top_5_compradores(usuarios);

            assert!(top_1.iter().any(|u| u.rol == RolUsuario::Comprador));
            assert!(!top_1.iter().any(|u| u.rol == RolUsuario::Vendedor));
        }

        #[test]
        fn test_top_5_compradores_ordena_por_reputacion() {
            let usuarios = vec![
                crear_usuario_test(AccountId::from([0x40; 32]), RolUsuario::Comprador, 10, 0),
                crear_usuario_test(AccountId::from([0x41; 32]), RolUsuario::Comprador, 50, 0),
                crear_usuario_test(AccountId::from([0x42; 32]), RolUsuario::Comprador, 30, 0),
            ];

            let top_5 = ReportesView::filtrar_top_5_compradores(usuarios);

            assert_eq!(top_5[0].reputacion_como_comprador, 50);
            assert_eq!(top_5[1].reputacion_como_comprador, 30);
            assert_eq!(top_5[2].reputacion_como_comprador, 10);
        }

        #[test]
        fn test_top_5_compradores_respeta_limite() {
            let usuarios: Vec<Usuario> = (0..8)
                .map(|i| crear_usuario_test(
                    AccountId::from([i as u8; 32]),
                    RolUsuario::Comprador,
                    100 - i as u32 * 10,
                    0,
                ))
                .collect();



            let top_5 = ReportesView::filtrar_top_5_compradores(usuarios);

            assert_eq!(top_5.len(), 5);
        }

        // =====================================================================
        // TESTS PARA productos_mas_vendidos - Agregación y ordenamiento
        // =====================================================================

        #[test]
        fn test_productos_mas_vendidos_agrega_ordenes() {
            let productos = vec![
                crear_producto_test(1, "Laptop", AccountId::from([0x10; 32])),
            ];
            let ordenes = vec![
                crear_orden_test(1, AccountId::from([0x11; 32]), AccountId::from([0x10; 32]), 1, 5, EstadoOrden::Recibida),
                crear_orden_test(2, AccountId::from([0x12; 32]), AccountId::from([0x10; 32]), 1, 3, EstadoOrden::Recibida),
            ];

            let reportes = ReportesView::calcular_productos_mas_vendidos(productos, ordenes);

            assert_eq!(reportes.len(), 1);
            assert_eq!(reportes[0].nombre_producto, "Laptop");
            assert_eq!(reportes[0].total_vendido, 8);
        }

        #[test]
        fn test_productos_mas_vendidos_ordena_descendente() {
            let productos = vec![
                crear_producto_test(1, "Mouse", AccountId::from([0x10; 32])),
                crear_producto_test(2, "Teclado", AccountId::from([0x10; 32])),
                crear_producto_test(3, "Monitor", AccountId::from([0x10; 32])),
            ];
            let ordenes = vec![
                crear_orden_test(1, AccountId::from([0x11; 32]), AccountId::from([0x10; 32]), 1, 10, EstadoOrden::Recibida),
                crear_orden_test(2, AccountId::from([0x11; 32]), AccountId::from([0x10; 32]), 2, 50, EstadoOrden::Recibida),
                crear_orden_test(3, AccountId::from([0x11; 32]), AccountId::from([0x10; 32]), 3, 30, EstadoOrden::Recibida),
            ];

            let reportes = ReportesView::calcular_productos_mas_vendidos(productos, ordenes);

            assert_eq!(reportes[0].nombre_producto, "Teclado");
            assert_eq!(reportes[0].total_vendido, 50);
            assert_eq!(reportes[1].total_vendido, 30);
            assert_eq!(reportes[2].total_vendido, 10);
        }

        #[test]
        fn test_productos_mas_vendidos_respeta_limite_5() {
            let productos: Vec<Producto> = (1..=10)
                .map(|i| crear_producto_test(i, &format!("Producto{}", i), AccountId::from([0x10; 32])))
                .collect();
            
            let ordenes: Vec<Orden> = (1..=10)
                .map(|i| crear_orden_test(i, AccountId::from([0x11; 32]), AccountId::from([0x10; 32]), i, 100 - i * 5, EstadoOrden::Recibida))
                .collect();

            let reportes = ReportesView::calcular_productos_mas_vendidos(productos, ordenes);

            assert_eq!(reportes.len(), 5);
            assert_eq!(reportes[0].total_vendido, 95);
        }

        #[test]
        fn test_productos_mas_vendidos_sin_ordenes() {
            let productos = vec![crear_producto_test(1, "Laptop", AccountId::from([0x10; 32]))];
            let ordenes = Vec::new();

            let reportes = ReportesView::calcular_productos_mas_vendidos(productos, ordenes);

            assert_eq!(reportes.len(), 0);
        }

        // =====================================================================
        // TESTS PARA contar_ordenes_usuario - Filtrado y conteo
        // =====================================================================

        #[test]
        fn test_contar_ordenes_usuario_filtra_por_comprador() {
            let usuario_a = AccountId::from([0x50; 32]);
            let usuario_b = AccountId::from([0x51; 32]);

            let ordenes = vec![
                crear_orden_test(1, usuario_a, usuario_b, 1, 1, EstadoOrden::Pendiente),
                crear_orden_test(2, usuario_a, usuario_b, 2, 1, EstadoOrden::Enviada),
                crear_orden_test(3, usuario_b, usuario_a, 3, 1, EstadoOrden::Recibida),
            ];

            let count_a = ReportesView::calcular_ordenes_usuario(ordenes, usuario_a);

            assert_eq!(count_a, 3);
        }

        #[test]
        fn test_contar_ordenes_usuario_filtra_por_vendedor() {
            let usuario_a = AccountId::from([0x60; 32]);
            let usuario_b = AccountId::from([0x61; 32]);
            let usuario_c = AccountId::from([0x62; 32]);

            let ordenes = vec![
                crear_orden_test(1, usuario_a, usuario_b, 1, 1, EstadoOrden::Pendiente),
                crear_orden_test(2, usuario_c, usuario_b, 2, 1, EstadoOrden::Enviada),
                crear_orden_test(3, usuario_a, usuario_c, 3, 1, EstadoOrden::Recibida),
            ];

            let count_b = ReportesView::calcular_ordenes_usuario(ordenes, usuario_b);

            assert_eq!(count_b, 2);
        }

        #[test]
        fn test_contar_ordenes_usuario_usuario_sin_ordenes() {
            let usuario_a = AccountId::from([0x70; 32]);
            let usuario_b = AccountId::from([0x71; 32]);
            let usuario_inexistente = AccountId::from([0x99; 32]);

            let ordenes = vec![
                crear_orden_test(1, usuario_a, usuario_b, 1, 1, EstadoOrden::Pendiente),
            ];

            let count = ReportesView::calcular_ordenes_usuario(ordenes, usuario_inexistente);

            assert_eq!(count, 0);
        }

        #[test]
        fn test_contar_ordenes_usuario_conversion_a_u32() {
            let usuario = AccountId::from([0x80; 32]);
            let ordenes: Vec<Orden> = vec![
                crear_orden_test(1, usuario, AccountId::from([0x81; 32]), 1, 1, EstadoOrden::Pendiente),
                crear_orden_test(2, usuario, AccountId::from([0x82; 32]), 2, 1, EstadoOrden::Enviada),
            ];

            let count = ReportesView::calcular_ordenes_usuario(ordenes, usuario);

            assert_eq!(count, 2);
        }

        // =====================================================================
        // TESTS DE WRAPPER (Métodos Públicos con Fetchers Simulados)
        // =====================================================================

        #[test]
        fn test_wrapper_top_5_vendedores() {
            let contract = ReportesView::nuevo(AccountId::from([0x01; 32]));
            let vendedores = contract.top_5_vendedores();
            // El mock devuelve 1 vendedor (0x90) y 1 comprador (0x91)
            // Solo debe retornar el vendedor
            assert_eq!(vendedores.len(), 1);
            assert_eq!(vendedores[0].rol, RolUsuario::Vendedor);
        }

        #[test]
        fn test_wrapper_top_5_compradores() {
            let contract = ReportesView::nuevo(AccountId::from([0x01; 32]));
            let compradores = contract.top_5_compradores();
            // El mock devuelve 1 vendedor y 1 comprador
            assert_eq!(compradores.len(), 1);
            assert_eq!(compradores[0].rol, RolUsuario::Comprador);
        }

        #[test]
        fn test_wrapper_productos_mas_vendidos() {
            let contract = ReportesView::nuevo(AccountId::from([0x01; 32]));
            let reportes = contract.productos_mas_vendidos();
            // El mock devuelve 1 producto y 1 orden recibida de 5 unidades
            assert_eq!(reportes.len(), 1);
            assert_eq!(reportes[0].total_vendido, 5);
        }

        #[test]
        fn test_wrapper_contar_ordenes_usuario() {
            let contract = ReportesView::nuevo(AccountId::from([0x01; 32]));
            // El mock devuelve 1 orden donde comprador es 0x91
            let count = contract.obtener_total_ordenes_usuario(AccountId::from([0x91; 32]));
            assert_eq!(count, 1);
        }
    }
}

#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests {
    use super::*;
    use ink_e2e::ContractsBackend;
    use marketplace_principal::{
        internal::MarketplaceRef,
        RolUsuario as MarketRol
    };
    use crate::reportes_view::ReportesViewRef;
    
    // Construcción de llamadas a bajo nivel
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::env::DefaultEnvironment;

    type ResultadoE2E<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[ink_e2e::test]
    async fn test_integracion_reportes(mut cliente: ink_e2e::Client<C, E>) -> ResultadoE2E<()> {
        // Alias para nombres argentinos
        let maria = ink_e2e::alice();
        let juan = ink_e2e::bob();

        // 1. Desplegar Marketplace
        let mut constructor_marketplace = MarketplaceRef::nuevo();
        let cuenta_marketplace = cliente
            .instantiate(
                "marketplace_principal",
                &maria,
                &mut constructor_marketplace,
            )
            .submit()
            .await
            .expect("Error al instanciar marketplace")
            .account_id;

        // 2. Desplegar ReportesView
        let mut constructor_reportes = ReportesViewRef::nuevo(cuenta_marketplace);
        let cuenta_reportes = cliente
            .instantiate(
                "reportes_view",
                &maria,
                &mut constructor_reportes,
            )
            .submit()
            .await
            .expect("Error al instanciar reportes_view")
            .account_id;

        // 3. Registrar Usuarios en Marketplace
        // Maria será Vendedora
        let registrar_maria = build_call::<DefaultEnvironment>()
            .call(cuenta_marketplace)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("registrar_usuario")))
                    .push_arg(MarketRol::Vendedor)
            )
            .returns::<()>();

        cliente
            .call(&maria, &registrar_maria)
            .submit()
            .await
            .expect("Falló registro de Maria");

        // Juan será Comprador
        let registrar_juan = build_call::<DefaultEnvironment>()
            .call(cuenta_marketplace)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("registrar_usuario")))
                    .push_arg(MarketRol::Comprador)
            )
            .returns::<()>();
            
        cliente
            .call(&juan, &registrar_juan)
            .submit()
            .await
            .expect("Falló registro de Juan");

        // 4. Verificar que ReportesView ve a los usuarios
        // Top 5 Vendedores
        let obtener_top_vendedores = build_call::<DefaultEnvironment>()
            .call(cuenta_reportes)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("top_5_vendedores")))
            )
            .returns::<Vec<Usuario>>();
            
        let top_vendedores = cliente
            .call(&maria, &obtener_top_vendedores)
            .dry_run()
            .await?
            .return_value();
        
        assert!(top_vendedores.iter().any(|u| u.rol == RolUsuario::Vendedor));

        // 5. Crear Orden
        // Maria publica producto
        let publicar_producto = build_call::<DefaultEnvironment>()
            .call(cuenta_marketplace)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("publicar_producto")))
                    .push_arg(String::from("Laptop"))
                    .push_arg(String::from("Laptop Gamer"))
                    .push_arg(1000u32)
                    .push_arg(10u32)
                    .push_arg(String::from("Electronica"))
            )
            .returns::<Result<(), marketplace_principal::SistemaError>>();

        cliente
            .call(&maria, &publicar_producto)
            .submit()
            .await
            .expect("Falló publicación de Maria");

        // Juan compra producto (id 0)
        let comprar_producto = build_call::<DefaultEnvironment>()
            .call(cuenta_marketplace)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("crear_orden")))
                    .push_arg(0u32) // id
                    .push_arg(1u32) // cantidad
            )
            .returns::<Result<u32, marketplace_principal::SistemaError>>();

        cliente
            .call(&juan, &comprar_producto)
            .submit()
            .await
            .expect("Falló compra de Juan");

        // 6. Verificar conteo en ReportesView
        // Obtenemos AccountId de Juan para consultarlo
        let id_cuenta_juan = ink::primitives::AccountId::from(juan.public_key().to_account_id().0);
        
        let contar_ordenes = build_call::<DefaultEnvironment>()
            .call(cuenta_reportes)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("obtener_total_ordenes_usuario")))
                    .push_arg(id_cuenta_juan)
            )
            .returns::<u32>();

        let conteo = cliente
            .call(&maria, &contar_ordenes)
            .dry_run()
            .await?
            .return_value();

        assert_eq!(conteo, 1);

        Ok(())
    }
}
