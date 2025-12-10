#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(unexpected_cfgs)]

    use ink::prelude::string::String;

    /// Roles posibles de usuario.
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum RolUsuario {
        Comprador,
        Vendedor,
        Ambos,
    }

    /// Estados posibles de una orden.
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum EstadoOrden {
        Pendiente,
        Enviada,
        Recibida,
        Cancelada,
    }

    /// Usuario del marketplace.
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Usuario {
        pub direccion: AccountId,
        pub rol: RolUsuario,
        pub reputacion_como_comprador: u32,
        pub reputacion_como_vendedor: u32,
    }

    /// Estructura de Producto.
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Producto {
        pub id: u32,
        pub nombre: String,
        pub descripcion: String,
        pub precio: Balance,
        pub cantidad: u32,
        pub categoria: String,
        pub vendedor: AccountId,
    }

    impl Producto {
        /// Nuevo producto.
        pub fn nuevo(id: u32, nombre: String, descripcion: String, precio: Balance, cantidad: u32, categoria: String, vendedor: AccountId) -> Self {
            Self { id, nombre, descripcion, precio, cantidad, categoria, vendedor }
        }
    }

    /// Representa una orden.
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

    impl Orden {
        /// Nueva orden (pendiente).
        pub fn nueva(id: u32, comprador: AccountId, vendedor: AccountId, producto_id: u32, cantidad: u32) -> Self {
            Self {
                id, comprador, vendedor, producto_id, cantidad,
                estado: EstadoOrden::Pendiente,
                comprador_califico: false,
                vendedor_califico: false,
                comprador_solicita_cancelacion: false,
                vendedor_acepta_cancelacion: false,
            }
        }
    }

    /// Errores del sistema
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum SistemaError {
        CantidadInsuficiente,
        UsuarioNoRegistrado,
        ProductosVacios,
        NoEsRolCorrecto,
        EstadoInvalido,
        OrdenNoExiste,
        UsuarioExistente,
        StockInsuficiente,
        CalificacionInvalida,
        YaCalificado,
    }

    // TIPOS ALIAS NECESARIOS PORQUE ESTAMOS FUERA DEL MODULO CONTRACT
    pub type AccountId = ink::primitives::AccountId;
    pub type Balance = u128; 

    // --- 2. EL CONTRATO PRINCIPAL ---`

    #[ink::contract]
    pub mod internal {
        use super::*;
        use ink::storage::Mapping;
        use ink::prelude::string::String;
        use ink::prelude::vec::Vec;

        #[ink(event)]
        pub struct RolActualizado {
            pub cuenta: AccountId,
            pub rol_anterior: RolUsuario,
            pub rol_nuevo: RolUsuario,
        }

        #[ink(event)]
        pub struct ProductoPublicado {
            pub vendedor: AccountId,
            pub producto_id: u32,
        }

        #[ink(event)]
        pub struct CompradorCalifico {
            pub orden_id: u32,
            pub comprador: AccountId,
            pub vendedor: AccountId,
            pub calificacion: u32,
        }

        #[ink(event)]
        pub struct VendedorCalifico {
            pub orden_id: u32,
            pub vendedor: AccountId,
            pub comprador: AccountId,
            pub calificacion: u32,
        }

        /// # Contrato Marketplace
        ///
        /// Este contrato implementa un mercado descentralizado completo.
        /// Permite a los usuarios registrarse como Compradores, Vendedores o Ambos.
        ///
        /// ## Funcionalidades Principales
        /// - **Usuarios**: Registro y gestión de reputación.
        /// - **Productos**: Publicación y gestión de stock.
        /// - **Órdenes**: Ciclo de vida completo (Creación -> Envío -> Recepción -> Calificación).
        ///
        /// ## Almacenamiento
        /// El contrato utiliza `Mapping` para almacenar usuarios, productos y órdenes de manera eficiente (O(1)).
        ///
        /// ## Eventos
        /// Emite eventos para acciones críticas como `ProductoPublicado`, `RolActualizado`, y calificaciones.
        #[ink(storage)]
        pub struct Marketplace {
            /// Mapeo de usuarios registrados (por dirección).
            usuarios: Mapping<AccountId, Usuario>,
            /// Mapeo de productos publicados (ID -> Producto).
            productos: Mapping<u32, Producto>,
            /// Contador para IDs de productos.
            next_producto_id: u32,
            /// Mapeo de órdenes generadas (ID -> Orden).
            ordenes: Mapping<u32, Orden>,
            /// Contador para IDs de órdenes.
            next_orden_id: u32,
            /// Lista auxiliar de IDs para poder recorrer los usuarios
            lista_usuarios_ids: Vec<AccountId>,
        }

        impl Marketplace {
            /// Constructor inicial.
            #[ink(constructor)]
            pub fn nuevo() -> Self {
                Self {
                    usuarios: Mapping::default(),
                    productos: Mapping::default(),
                    next_producto_id: 0,
                    ordenes: Mapping::default(),
                    next_orden_id: 0,
                    lista_usuarios_ids: Vec::new(),
                }
            }
            // --- Getters para testing y verificación de estado ---
            /// Cantidad total de productos.
            #[ink(message)]
            pub fn obtener_cantidad_productos(&self) -> u32 {
                self.next_producto_id
            }

            /// Cantidad total de órdenes.
            #[ink(message)]
            pub fn obtener_cantidad_ordenes(&self) -> u32 {
                self.next_orden_id
            }

            /// Registra un nuevo usuario con el rol especificado.
            #[ink(message)]
            pub fn registrar_usuario(&mut self, rol: RolUsuario) -> Result<(), SistemaError> {
                self.registrar_usuario_interno(rol)
            }

            
            /// Verifica si una cuenta ya está registrada en el marketplace.
            ///
            /// # Parámetros
            /// * `usuario` - La dirección de la cuenta (AccountId) a verificar.
            ///
            /// # Retorno
            /// * `true` si el usuario existe en el mapping de usuarios.
            /// * `false` en caso contrario.
            #[ink(message)]
            pub fn esta_registrado(&self, usuario: AccountId) -> bool {
            self.usuarios.contains(&usuario)
        }

        
            /// Obtiene los datos completos de un usuario registrado.
            ///
            /// # Parámetros
            /// * `usuario` - La dirección de la cuenta a buscar.
            ///
            /// # Retorno
            /// * `Some(Usuario)` conteniendo la info del usuario si existe.
            /// * `None` si el usuario no está registrado.
            #[ink(message)]
            pub fn obtener_usuario(&self, usuario: AccountId) -> Option<Usuario> {
            self.usuarios.get(&usuario)
        }

        /// Registro interno de usuario.
        fn registrar_usuario_interno(&mut self, rol: RolUsuario) -> Result<(), SistemaError> {
            let usuario_llamador = self.env().caller();
            // Verifica si el usuario es existente
            if self.usuarios.contains(&usuario_llamador) { 
                return Err(SistemaError::UsuarioExistente);
            }
            // Si no existe, crea un nuevo usuario
            let nuevo_usuario = Usuario {
                direccion: usuario_llamador,
                rol,
                reputacion_como_comprador: 0,
                reputacion_como_vendedor: 0,
            };
            self.usuarios.insert(usuario_llamador, &nuevo_usuario);
            self.lista_usuarios_ids.push(usuario_llamador);
            
            Ok(())
        }


            /// Modifica el rol de un usuario existente.
            ///
            /// # Parámetros
            /// * `nuevo_rol` - El rol al que se desea cambiar (`Comprador`, `Vendedor`, `Ambos`).
            ///
            /// # Errores
            /// * `UsuarioNoRegistrado`: Quien llama no tiene cuenta.
            /// * `NoEsRolCorrecto`: La transición no es válida (e.g. Vendedor -> Comprador si no es permitido directo) o es el mismo rol actual.
            #[ink(message)]
            pub fn modificar_rol_usuario(&mut self,nuevo_rol: RolUsuario,) -> Result<(), SistemaError> {
            self.modificar_rol_usuario_interno(nuevo_rol)
        }

        fn modificar_rol_usuario_interno(&mut self,nuevo_rol: RolUsuario,) -> Result<(), SistemaError> {
            let usuario_llamador = self.env().caller();
            // Verifica que el usuario esté registrado
            // Obtener el usuario directamente para trabajar con su estado actual
            let mut usuario = self.usuarios.get(&usuario_llamador)
                .ok_or(SistemaError::UsuarioNoRegistrado)?;

            // No se permite "cambiar" al mismo rol
            if usuario.rol == nuevo_rol {
                return Err(SistemaError::NoEsRolCorrecto);
            }

            // Validar transiciones permitidas
            match usuario.rol {
                // Usuarios con rol Ambos pueden cambiar a cualquier otro rol (ya filtrado el mismo)
                RolUsuario::Ambos => { /* permitido */ },
                // Comprador solo puede cambiar a Vendedor
                RolUsuario::Comprador => {
                    if nuevo_rol != RolUsuario::Vendedor {
                        return Err(SistemaError::NoEsRolCorrecto);
                    }
                }
                // Vendedor solo puede cambiar a Comprador
                RolUsuario::Vendedor => {
                    if nuevo_rol != RolUsuario::Comprador {
                        return Err(SistemaError::NoEsRolCorrecto);
                    }
                }
            }

            // Guarda rol anterior para el evento y actualiza el mapping
            let rol_anterior = usuario.rol.clone();
            usuario.rol = nuevo_rol.clone();
            self.usuarios.insert(usuario_llamador, &usuario);

            // Evento
            self.env().emit_event(RolActualizado {
                cuenta: usuario_llamador,
                rol_anterior,
                rol_nuevo: nuevo_rol,
            });

            Ok(())
        }


            /// Publica un nuevo producto en el catálogo.
            ///
            /// # Parámetros
            /// * `nombre` - Nombre del producto.
            /// * `descripcion` - Breve descripción.
            /// * `precio` - Costo unitario.
            /// * `cantidad` - Stock inicial disponible.
            /// * `categoria` - Categoría para agrupación.
            ///
            /// # Errores
            /// * `UsuarioNoRegistrado`: El caller no existe.
            /// * `NoEsRolCorrecto`: El caller no es Vendedor o Ambos.
            /// * `CantidadInsuficiente`: El stock inicial es 0.
            /// * `ProductosVacios`: Error interno al generar ID.
            #[ink(message)]
            pub fn publicar_producto(
                &mut self,
                nombre: String,
                descripcion: String,
                precio: Balance,
                cantidad: u32,
                categoria: String,
            ) -> Result<(), SistemaError> {
            self.crear_producto_seguro(nombre, descripcion, precio, cantidad, categoria)
        }

        /// Valida y agrega el producto.
        fn crear_producto_seguro(
            &mut self,
            nombre: String,
            descripcion: String,
            precio: Balance,
            cantidad: u32,
            categoria: String,
        ) -> Result<(), SistemaError> {
            let vendedor = self.env().caller();
            // Verifica que el vendedor esté registrado y tenga el rol adecuado
            self.verificar_registro(vendedor)?;
            self.verificar_rol(vendedor, RolUsuario::Vendedor)?;
            // Verifica que la cantidad sea válida
            self.verificar_cantidad(cantidad)?;
            // Agrega el producto al marketplace
            self.agregar_producto(nombre, descripcion, precio, cantidad, categoria, vendedor)
        }

        

            /// Lista todos los productos publicados por el usuario que llama (sus propios productos).
            ///
            /// # Retorno
            /// * `Ok(Vec<Producto>)`: Lista de productos.
            ///
            /// # Errores
            /// * `UsuarioNoRegistrado` o `NoEsRolCorrecto` si no es vendedor.
            /// * `ProductosVacios`: Si no tiene productos publicados.
            #[ink(message)]
            pub fn listar_mis_productos(&self) -> Result<Vec<Producto>, SistemaError> {
            let yo = self.env().caller();
            self.listar_productos_interno(yo)
        }

        /// Interna: valida que `vendedor` exista y tenga rol de Vendedor/Ambos,
        /// y devuelve la lista de sus productos o un error específico.
        fn listar_productos_interno(&self, vendedor: AccountId) -> Result<Vec<Producto>, SistemaError> {
            // Valida registro + rol
            self.verificar_rol(vendedor, RolUsuario::Vendedor)?;

            let mut productos_vendedor = Vec::new();
            for id in 0..self.next_producto_id {
                if let Some(prod) = self.productos.get(id) {
                    if prod.vendedor == vendedor {
                        productos_vendedor.push(prod);
                    }
                }
            }

            if productos_vendedor.is_empty() {
                return Err(SistemaError::ProductosVacios);
            }
            Ok(productos_vendedor)
        }


            /// Muestra los productos de un vendedor específico.
            ///
            /// # Parámetros
            /// * `vendedor` - AccountId del vendedor a consultar.
            ///
            /// # Errores
            /// * `ProductosVacios`: Si el vendedor no tiene productos activos.
            #[ink(message)]
            pub fn listar_productos_por_vendedor(&self, vendedor: AccountId) -> Result<Vec<Producto>, SistemaError> {
            self.listar_productos_por_vendedor_interno(vendedor)
        }

        pub fn listar_productos_por_vendedor_interno(&self, vendedor: AccountId) -> Result<Vec<Producto>, SistemaError> {
            let mut productos = Vec::new();
            for id in 0..self.next_producto_id {
                if let Some(prod) = self.productos.get(id) {
                    if prod.vendedor == vendedor {
                        productos.push(prod);
                    }
                }
            }
            if productos.is_empty() {
                return Err(SistemaError::ProductosVacios);
            }
            Ok(productos)
        }
        
            /// Genera una nueva orden de compra.
            ///
            /// # Parámetros
            /// * `producto_id` - ID del producto a comprar.
            /// * `cantidad` - Unidades a adquirir.
            ///
            /// # Retorno
            /// * `Ok(u32)`: El ID de la orden generada.
            ///
            /// # Errores
            /// * `StockInsuficiente`: El producto no tiene tantas unidades.
            /// * `ProductosVacios`: El producto no existe.
            /// * `NoEsRolCorrecto`: El comprador no tiene el rol adecuado.
            #[ink(message)]
            pub fn crear_orden(&mut self, producto_id: u32, cantidad: u32) -> Result<u32, SistemaError> {
            let comprador = self.env().caller();
            if !self.usuarios.contains(&comprador) { return Err(SistemaError::UsuarioNoRegistrado); }
            let u = self.usuarios.get(&comprador).unwrap();
            match u.rol {
                RolUsuario::Comprador | RolUsuario::Ambos => {},
                _ => return Err(SistemaError::NoEsRolCorrecto),
            }

            if cantidad == 0 { return Err(SistemaError::CantidadInsuficiente); }

            // Obtener producto y validar stock
            let mut prod = self.productos.get(producto_id).ok_or(SistemaError::ProductosVacios)?;
            if prod.cantidad < cantidad { return Err(SistemaError::StockInsuficiente); }
            let vendedor_addr = prod.vendedor;

            // Actualizar stock
            prod.cantidad = prod.cantidad.saturating_sub(cantidad);
            self.productos.insert(producto_id, &prod);

            // Crear orden
            let id = self.next_orden_id;
            // Incrementar ID de orden
             self.next_orden_id = self.next_orden_id.checked_add(1).ok_or(SistemaError::ProductosVacios)?;

            let nueva = Orden::nueva(id, comprador, vendedor_addr, producto_id, cantidad);
            self.ordenes.insert(id, &nueva);
            Ok(id)
        }
        

            /// Cambia el estado de una orden a `Enviada`.
            ///
            /// # Requisitos
            /// * Solo el **Vendedor** de la orden puede ejecutar esto.
            /// * La orden debe estar en estado `Pendiente`.
            #[ink(message)]
            pub fn marcar_orden_como_enviada(&mut self, orden_id: u32) -> Result<(), SistemaError> {
            self.actualizar_estado_orden(orden_id, EstadoOrden::Enviada)
        }


            /// Cambia el estado de una orden a `Recibida`.
            ///
            /// # Requisitos
            /// * Solo el **Comprador** de la orden puede ejecutar esto.
            /// * La orden debe estar en estado `Enviada`.
            #[ink(message)]
            pub fn marcar_como_recibida(&mut self, orden_id: u32) -> Result<(), SistemaError> {
            self.actualizar_estado_orden(orden_id, EstadoOrden::Recibida)
        }

        /// Actualiza estado de la orden.
        fn actualizar_estado_orden(&mut self, orden_id: u32, nuevo_estado: EstadoOrden) -> Result<(), SistemaError> {
            let caller = self.env().caller();
            self.verificar_registro(caller)?;
            
            let mut orden = self.ordenes.get(orden_id).ok_or(SistemaError::OrdenNoExiste)?;
            
            // Verificar permiso (usamos referencia a la copia)
            self.verificar_permiso_orden(caller, &orden, &nuevo_estado)?;
            
            orden.estado = nuevo_estado;
            self.ordenes.insert(orden_id, &orden);
            Ok(())
        }

            /// Permite al comprador calificar su experiencia con el vendedor.
            ///
            /// # Parámetros
            /// * `orden_id` - ID de la orden finalizada.
            /// * `calificacion` - Valor entero entre 1 y 5.
            ///
            /// # Errores
            /// * `CalificacionInvalida`: Si el valor no está entre 1 y 5.
            /// * `EstadoInvalido`: Si la orden no está `Recibida`.
            /// * `YaCalificado`: Si ya se emitió voto para esta orden.
            #[ink(message)]
            pub fn calificar_vendedor(&mut self, orden_id: u32, calificacion: u32) -> Result<(), SistemaError> {
            let caller = self.env().caller();
            
            if !(1..=5).contains(&calificacion) { return Err(SistemaError::CalificacionInvalida); }

            let mut orden = self.ordenes.get(orden_id).ok_or(SistemaError::OrdenNoExiste)?;
            if orden.comprador != caller { return Err(SistemaError::NoEsRolCorrecto); }
            if orden.estado != EstadoOrden::Recibida { return Err(SistemaError::EstadoInvalido); }
            if orden.comprador_califico { return Err(SistemaError::YaCalificado); }

            orden.comprador_califico = true;
            self.ordenes.insert(orden_id, &orden);
            
            // Actualizar reputación
            let vendedor_addr = orden.vendedor;
            if let Some(mut v) = self.usuarios.get(&vendedor_addr) {
                v.reputacion_como_vendedor = v.reputacion_como_vendedor.saturating_add(calificacion);
                self.usuarios.insert(vendedor_addr, &v);
            }
            
            self.env().emit_event(CompradorCalifico { orden_id, comprador: caller, vendedor: vendedor_addr, calificacion });
            Ok(())
        }



        
            /// Permite al vendedor calificar al comprador.
            ///
            /// # Parámetros
            /// * `orden_id` - ID de la orden finalizada.
            /// * `calificacion` - Valor entero entre 1 y 5.
            ///
            /// # Errores
            /// * `CalificacionInvalida`: Si el valor no está entre 1 y 5.
            /// * `EstadoInvalido`: Si la orden no está `Recibida`.
            /// * `YaCalificado`: Si ya se emitió voto.
            #[ink(message)]
            pub fn calificar_comprador(&mut self, orden_id: u32, calificacion: u32) -> Result<(), SistemaError> {
            let caller = self.env().caller();
            
            // Validación de rango manual
            if !(1..=5).contains(&calificacion) { return Err(SistemaError::CalificacionInvalida); }

            let mut orden = self.ordenes.get(orden_id).ok_or(SistemaError::OrdenNoExiste)?;
            if orden.vendedor != caller { return Err(SistemaError::NoEsRolCorrecto); }
            if orden.estado != EstadoOrden::Recibida { return Err(SistemaError::EstadoInvalido); }
            if orden.vendedor_califico { return Err(SistemaError::YaCalificado); }

            orden.vendedor_califico = true;
            self.ordenes.insert(orden_id, &orden);

            let comprador_addr = orden.comprador;
            if let Some(mut c) = self.usuarios.get(&comprador_addr) {
                c.reputacion_como_comprador = c.reputacion_como_comprador.saturating_add(calificacion);
                self.usuarios.insert(comprador_addr, &c);
            }

            self.env().emit_event(VendedorCalifico { orden_id, vendedor: caller, comprador: comprador_addr, calificacion });
            Ok(())
        }


        /// Verifica si un usuario está registrado.
        fn verificar_registro(&self, usuario: AccountId) -> Result<(), SistemaError> {
            if !self.usuarios.contains(&usuario) { // Cambia contains_key por contains
                Err(SistemaError::UsuarioNoRegistrado)
            } else {
                Ok(())
            }
        }

        /// Verifica si el usuario tiene el rol requerido.
        fn verificar_rol(&self, usuario: AccountId, rol_requerido: RolUsuario) -> Result<(), SistemaError> {
            let usuario_data = self.usuarios.get(&usuario)
                .ok_or(SistemaError::UsuarioNoRegistrado)?;

            match (usuario_data.rol, rol_requerido) {
                // Solo usuarios con rol Comprador pueden crear órdenes
                (RolUsuario::Comprador, RolUsuario::Comprador) => Ok(()),
                // Solo usuarios con rol Vendedor pueden publicar productos
                (RolUsuario::Vendedor, RolUsuario::Vendedor) => Ok(()),
                // Usuarios con rol Ambos pueden hacer ambas acciones
                (RolUsuario::Ambos, _) => Ok(()),
                _ => Err(SistemaError::NoEsRolCorrecto),
            }
        }


        fn verificar_cantidad(&self, cantidad: u32) -> Result<(), SistemaError> {
            if cantidad == 0 {
                Err(SistemaError::CantidadInsuficiente)
            } else {
                Ok(())
            }
        }


        fn agregar_producto(
            &mut self,
            nombre: String,
            descripcion: String,
            precio: Balance,
            cantidad: u32,
            categoria: String,
            vendedor: AccountId,
        ) -> Result<(), SistemaError> {
            let id = self.next_producto_id;
            self.next_producto_id = self.next_producto_id.checked_add(1).ok_or(SistemaError::ProductosVacios)?;

            let nuevo = Producto::nuevo(id, nombre, descripcion, precio, cantidad, categoria, vendedor);
            self.productos.insert(id, &nuevo);

            // Evento de publicación
            self.env().emit_event(ProductoPublicado { vendedor, producto_id: id });

            Ok(())
        }


        fn verificar_permiso_orden(
            &self,
            caller: AccountId,
            orden: &Orden,
            nuevo_estado: &EstadoOrden
        ) -> Result<(), SistemaError> {
            match nuevo_estado {
                EstadoOrden::Enviada if caller != orden.vendedor => Err(SistemaError::NoEsRolCorrecto),
                EstadoOrden::Recibida if caller != orden.comprador => Err(SistemaError::NoEsRolCorrecto),
                _ => self.verificar_transicion_estado(&orden.estado, nuevo_estado),
            }
        }

        /// Verifica que la transición de estado de la orden sea válida.
        fn verificar_transicion_estado(
            &self,
            actual: &EstadoOrden,
            nuevo: &EstadoOrden
        ) -> Result<(), SistemaError> {
            match (actual, nuevo) {
                (EstadoOrden::Pendiente, EstadoOrden::Enviada) => Ok(()),
                (EstadoOrden::Enviada, EstadoOrden::Recibida) => Ok(()),
                _ => Err(SistemaError::EstadoInvalido),
            }
        }

            /// Inicia o completa el proceso de cancelación.
            ///
            /// Si el Comprador llama, marca su solicitud.
            /// Si el Vendedor llama, marca su aceptación.
            /// Cuando **ambos** han aceptado, la orden pasa a `Cancelada` y se devuelve el stock.
            ///
            /// # Errores
            /// * `EstadoInvalido`: Solo se pueden cancelar órdenes `Pendiente`.
            #[ink(message)]
            pub fn solicitar_cancelacion_orden(&mut self, orden_id: u32) -> Result<(), SistemaError> {
            let caller = self.env().caller();
            if !self.usuarios.contains(&caller) { return Err(SistemaError::UsuarioNoRegistrado); }

            let (prod_id, cant, cancelar) = {
                let mut orden = self.ordenes.get(orden_id).ok_or(SistemaError::OrdenNoExiste)?;
                if orden.estado != EstadoOrden::Pendiente { return Err(SistemaError::EstadoInvalido); }

                if caller == orden.comprador { orden.comprador_solicita_cancelacion = true; }
                else if caller == orden.vendedor { orden.vendedor_acepta_cancelacion = true; }
                else { return Err(SistemaError::NoEsRolCorrecto); }
                
                self.ordenes.insert(orden_id, &orden);

                (orden.producto_id, orden.cantidad, orden.comprador_solicita_cancelacion && orden.vendedor_acepta_cancelacion)
            };

            if cancelar {
                let mut orden = self.ordenes.get(orden_id).ok_or(SistemaError::OrdenNoExiste)?;
                orden.estado = EstadoOrden::Cancelada;
                self.ordenes.insert(orden_id, &orden);
                
                if let Some(mut p) = self.productos.get(prod_id) {
                    p.cantidad = p.cantidad.saturating_add(cant);
                    self.productos.insert(prod_id, &p);
                }
            }
            Ok(())
        }


            /// Retorna todos los productos disponibles.
            #[ink(message)]
            pub fn obtener_todos_los_productos(&self) -> Vec<Producto> {
            let mut productos = Vec::new();
            for id in 0..self.next_producto_id {
                if let Some(prod) = self.productos.get(id) {
                    productos.push(prod);
                }
            }
            productos
        }

        /// Retorna todas las órdenes.
        #[ink(message)]
        pub fn obtener_todas_las_ordenes(&self) -> Vec<Orden> {
            let mut ordenes = Vec::new();
            for id in 0..self.next_orden_id {
                if let Some(orden) = self.ordenes.get(id) {
                    ordenes.push(orden);
                }
            }
            ordenes
        }

        /// Retorna todos los usuarios.
        #[ink(message)]
        pub fn obtener_todos_los_usuarios(&self) -> Vec<Usuario> {
            let mut lista_completa: Vec<Usuario> = Vec::new();
            
            for id in &self.lista_usuarios_ids {
                if let Some(usuario) = self.usuarios.get(id) {
                    lista_completa.push(usuario);
                }
            }
            
            lista_completa
        }
    }


    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        fn setup_contract_con_vendedor() -> Marketplace {
            let mut contrato = Marketplace::nuevo();
            let caller = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(caller);
            let usuario = Usuario {
                direccion: caller,
                rol: RolUsuario::Vendedor,
                reputacion_como_comprador: 0,
                reputacion_como_vendedor: 0,
            };
            contrato.usuarios.insert(caller, &usuario);
            contrato
        }
        
     
        #[ink::test]
        fn registrar_usuario_comprador_ok() {
            let mut contrato = Marketplace::nuevo();

      
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            test::set_caller::<ink::env::DefaultEnvironment>(maria);

            // Llamamos a la función registrar_usuario con el rol de comprador
            let resultado = contrato.registrar_usuario(RolUsuario::Comprador);

            // Verificamos que devuelva OK
            assert_eq!(resultado, Ok(()));

            // Obtenemos el usuario usando la dirección del caller
            let usuario_registrado = contrato.usuarios.get(&maria);

            // Confirmamos si se guardó el usuario
            assert!(usuario_registrado.is_some());

            // Verificamos los datos
            let usuario = usuario_registrado.unwrap();
            assert_eq!(usuario.rol, RolUsuario::Comprador);
            assert_eq!(usuario.reputacion_como_comprador, 0);
            assert_eq!(usuario.reputacion_como_vendedor, 0);
        }

        #[ink::test]
        fn registrar_usuario_vendedor_ok() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);

            // Llamamos a la función registrar_usuario con el rol de vendedor
            let resultado = contrato.registrar_usuario(RolUsuario::Vendedor);

            // Verificamos que devuelva OK
            assert_eq!(resultado, Ok(()));

            // Obtenemos el usuario usando la dirección del caller
            let usuario_registrado = contrato.usuarios.get(&juan);

            // Confirmamos si se guardó el usuario
            assert!(usuario_registrado.is_some());

            // Verificamos los datos
            let usuario = usuario_registrado.unwrap();
            assert_eq!(usuario.rol, RolUsuario::Vendedor);
            assert_eq!(usuario.reputacion_como_comprador, 0);
            assert_eq!(usuario.reputacion_como_vendedor, 0);
        }

        #[ink::test]
        fn registrar_usuario_ambos_ok() {
            let mut contrato = Marketplace::nuevo();

            // Simulamos que el caller es "Carlos"
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let carlos = cuentas.charlie;
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);

            // Llamamos a la función registrar_usuario con el rol de ambos
            let resultado = contrato.registrar_usuario(RolUsuario::Ambos);

            // Verificamos que devuelva OK
            assert_eq!(resultado, Ok(()));

            // Obtenemos el usuario usando la dirección del caller
            let usuario_registrado = contrato.usuarios.get(&carlos);

            // Confirmamos si se guardó el usuario
            assert!(usuario_registrado.is_some());

            // Verificamos los datos
            let usuario = usuario_registrado.unwrap();
            assert_eq!(usuario.rol, RolUsuario::Ambos);
            assert_eq!(usuario.reputacion_como_comprador, 0);
            assert_eq!(usuario.reputacion_como_vendedor, 0);
        }

        #[ink::test]
        fn registrar_usuario_duplicado_falla() {
            let mut contrato = Marketplace::nuevo();

            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

            // Primer registro
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Segundo registro debería fallar porque ya está registrado
            let resultado = contrato.registrar_usuario(RolUsuario::Vendedor);
            assert_eq!(resultado, Err(SistemaError::UsuarioExistente));
        }

        // --- Modificación de roles ---
        #[ink::test]
        fn modificar_rol_usuario_comprador_a_vendedor_ok() {
            let mut contrato = setup_contract_con_vendedor();

            // Cambia el caller a un usuario registrado como Comprador
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Modifica el rol a Vendedor
            let resultado = contrato.modificar_rol_usuario(RolUsuario::Vendedor);
            assert!(resultado.is_ok());

            // Verifica que el rol se haya actualizado correctamente
            let usuario = contrato.obtener_usuario(maria).unwrap();
            assert_eq!(usuario.rol, RolUsuario::Vendedor);
        }

        #[ink::test]
        fn modificar_rol_usuario_vendedor_a_comprador_ok() {
            let mut contrato = setup_contract_con_vendedor();

            // Cambia el caller a un usuario registrado como Vendedor
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Vendedor);

            // Modifica el rol a Comprador
            let resultado = contrato.modificar_rol_usuario(RolUsuario::Comprador);
            assert!(resultado.is_ok());

            // Verifica que el rol se haya actualizado correctamente
            let usuario = contrato.obtener_usuario(juan).unwrap();
            assert_eq!(usuario.rol, RolUsuario::Comprador);
        }

        #[ink::test]
        fn modificar_rol_usuario_ambos_a_comprador_ok() {
            let mut contrato = setup_contract_con_vendedor();

            // Cambia el caller a un usuario registrado como Ambos
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let carlos = cuentas.charlie;
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            let _ = contrato.registrar_usuario(RolUsuario::Ambos);

            // Modifica el rol a Comprador
            let resultado = contrato.modificar_rol_usuario(RolUsuario::Comprador);
            assert!(resultado.is_ok());

            // Verifica que el rol se haya actualizado correctamente
            let usuario = contrato.obtener_usuario(carlos).unwrap();
            assert_eq!(usuario.rol, RolUsuario::Comprador);
        }

        #[ink::test]
        fn emite_evento_rol_actualizado() {
            let mut c = Marketplace::nuevo();
            let cuentas = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(maria);

            c.registrar_usuario(RolUsuario::Comprador).unwrap();

            // Grabamos eventos durante la llamada que cambia el rol
            c.modificar_rol_usuario(RolUsuario::Vendedor).unwrap();
            let eventos = ink::env::test::recorded_events().collect::<Vec<_>>();
            assert!(!eventos.is_empty(), "Debe emitirse al menos un evento");
        }


        #[ink::test]
        fn modificar_rol_usuario_no_registrado_falla() {
            let mut contrato = Marketplace::nuevo();

            // Cambia el caller a un usuario no registrado
            let caller = AccountId::from([0x05; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(caller);

            // Intenta modificar el rol
            let resultado = contrato.modificar_rol_usuario(RolUsuario::Vendedor);
            assert!(matches!(resultado, Err(SistemaError::UsuarioNoRegistrado)));
        }

        #[ink::test]
        fn modificar_rol_usuario_mismo_rol_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Cambia el caller a un usuario registrado como Comprador
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Intenta cambiar a Comprador nuevamente
            let resultado = contrato.modificar_rol_usuario(RolUsuario::Comprador);
            assert!(matches!(resultado, Err(SistemaError::NoEsRolCorrecto)));
        }

        #[ink::test]
        fn modificar_rol_usuario_no_puede_cambiar_a_vendedor_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Cambia el caller a un usuario registrado como Vendedor
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Vendedor);

            // Intenta cambiar a Vendedor, lo cual no es permitido
            let resultado = contrato.modificar_rol_usuario(RolUsuario::Vendedor);
            assert!(matches!(resultado, Err(SistemaError::NoEsRolCorrecto)));
        }

        #[ink::test]
        fn modificar_rol_usuario_no_puede_cambiar_a_comprador_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Cambia el caller a un usuario registrado como Comprador
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Intenta cambiar a Comprador, lo cual no es permitido
            let resultado = contrato.modificar_rol_usuario(RolUsuario::Comprador);
            assert!(matches!(resultado, Err(SistemaError::NoEsRolCorrecto)));
        }   

        // --- Publicación de productos ---
        #[ink::test]
        fn publicar_producto_ok() {
            let mut contrato = setup_contract_con_vendedor();

            let resultado = contrato.publicar_producto(
                "Celular".to_string(),
                "Un buen celular".to_string(),
                1000,
                5,
                "Tecnología".to_string(),
            );

            assert!(resultado.is_ok());
            assert_eq!(contrato.obtener_cantidad_productos(), 1);

            let producto = contrato.productos.get(0).unwrap();
            assert_eq!(producto.nombre, "Celular");
            assert_eq!(producto.precio, 1000);
        }

        #[ink::test]
        fn publicar_producto_no_registrado_falla() {
            let mut contrato = Marketplace::nuevo();

            let caller = AccountId::from([0x02; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(caller);

            let resultado = contrato.publicar_producto(
                "Producto".to_string(),
                "Sin registro".to_string(),
                500,
                1,
                "Otros".to_string(),
            );

            assert!(matches!(resultado, Err(SistemaError::UsuarioNoRegistrado)));
        }

        #[ink::test]
        fn publicar_producto_no_es_vendedor_falla() {
            let mut contrato = Marketplace::nuevo();

            let caller = AccountId::from([0x03; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(caller);

            let usuario = Usuario {
                direccion: caller,
                rol: RolUsuario::Comprador, // Rol no válido para publicar productos
                reputacion_como_comprador: 0,
                reputacion_como_vendedor: 0,
            };
            contrato.usuarios.insert(caller, &usuario);

            let resultado = contrato.publicar_producto(
                "Producto".to_string(),
                "No autorizado".to_string(),
                100,
                2,
                "Otros".to_string(),
            );

            assert!(matches!(resultado, Err(SistemaError::NoEsRolCorrecto)));
        }

        #[ink::test]
        fn publicar_producto_cantidad_cero_falla() {
            let mut contrato = setup_contract_con_vendedor();

            let resultado = contrato.publicar_producto(
                "Producto".to_string(),
                "Cantidad cero".to_string(),
                100,
                0, // Cantidad inválida
                "Otros".to_string(),
            );

            assert!(matches!(resultado, Err(SistemaError::CantidadInsuficiente)));
        }

        // --- Listar productos ---
         #[ink::test]
        fn listar_interno_ok_para_vendedor() {
            let mut c = setup_contract_con_vendedor();

            // El caller ya está registrado como Vendedor por el helper
            c.publicar_producto("P1".into(), "D".into(), 100, 5, "Cat".into()).unwrap();
            c.publicar_producto("P2".into(), "D".into(), 200, 3, "Cat".into()).unwrap();

            let caller = ink::env::caller::<ink::env::DefaultEnvironment>();
            let v = c.listar_productos_interno(caller).unwrap();
            assert_eq!(v.len(), 2); // Debe devolver exactamente 2 productos del seller
            assert!(v.iter().all(|p| p.vendedor == caller)); //"Todos los productos deben pertenecer al seller
        }

        /// Error: usuario no registrado intenta listar.
        #[ink::test]
        fn listar_interno_falla_si_no_registrado() {
            let c = Marketplace::nuevo();
            let no_reg = AccountId::from([9u8; 32]);

            let res = c.listar_productos_interno(no_reg);
            assert!(matches!(res, Err(SistemaError::UsuarioNoRegistrado)));
        }

        /// Error: registrado como Comprador (no Vendedor/Ambos) intenta listar.
        #[ink::test]
        fn listar_interno_falla_si_no_es_vendedor() {
            let mut c = Marketplace::nuevo();

            let comprador = AccountId::from([2u8; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(comprador);
            c.registrar_usuario(RolUsuario::Comprador).unwrap();

            let res = c.listar_productos_interno(comprador);
            assert!(matches!(res, Err(SistemaError::NoEsRolCorrecto)));
        }

        /// Error: vendedor válido pero sin productos publicados.
        #[ink::test]
        fn listar_interno_falla_si_no_tiene_productos() {
            let c = setup_contract_con_vendedor();

            let caller = ink::env::caller::<ink::env::DefaultEnvironment>();
            let res = c.listar_productos_interno(caller);
            assert!(matches!(res, Err(SistemaError::ProductosVacios)));
        }


        // --- Compra y órdenes ---
        #[ink::test]
        fn crear_orden_ok() {
            let mut contrato = setup_contract_con_vendedor();

            // Publica un producto y obtiene el ID
            let _ = contrato.publicar_producto(
                "Laptop".to_string(),
                "Una laptop potente".to_string(),
                2000,
                10,
                "Tecnología".to_string(),
            );

            // Cambia el caller a un usuario comprador y regístralo
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // El producto publicado tendrá id = 0 (si es el primero)
            let resultado = contrato.crear_orden(0, 2);

            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();
            assert_eq!(contrato.obtener_cantidad_ordenes(), 1);

            let orden = contrato.ordenes.get(0).unwrap();
            assert_eq!(orden.id, orden_id);
            assert_eq!(orden.cantidad, 2);
            assert_eq!(orden.estado, EstadoOrden::Pendiente);
        }

        #[ink::test]
        fn crear_orden_no_registrado_falla() {
            let mut contrato = Marketplace::nuevo();

            let caller = AccountId::from([0x04; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(caller);

            let resultado = contrato.crear_orden(0, 1);

            assert!(matches!(resultado, Err(SistemaError::UsuarioNoRegistrado)));
        }

        #[ink::test]
        fn verificar_registro_antes_de_crear_orden() {
            let mut contrato = setup_contract_con_vendedor();

            // Publica un producto
            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            // Cambia el caller a un usuario NO registrado
            let nuevo_usuario = AccountId::from([0x99; 32]);
            // El comentario original decía "Cambia el caller...", mantenemos el estilo
            test::set_caller::<ink::env::DefaultEnvironment>(nuevo_usuario);

            // Verifica que el usuario no está registrado
            assert!(!contrato.esta_registrado(nuevo_usuario));
            assert!(contrato.obtener_usuario(nuevo_usuario).is_none());

            // Intenta crear una orden y falla porque no está registrado
            let resultado = contrato.crear_orden(0, 1);
            assert!(matches!(resultado, Err(SistemaError::UsuarioNoRegistrado)));

            // Registra al usuario como comprador
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Verifica que ahora está registrado
            assert!(contrato.esta_registrado(nuevo_usuario));
            let usuario_info = contrato.obtener_usuario(nuevo_usuario).unwrap();
            assert_eq!(usuario_info.rol, RolUsuario::Comprador);

            // Ahora puede crear una orden exitosamente
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
        }

        #[ink::test]
        fn crear_orden_no_es_comprador_falla() {
            let mut contrato = Marketplace::nuevo();

            let caller = AccountId::from([0x05; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(caller);

            let usuario = Usuario {
                direccion: caller,
                rol: RolUsuario::Vendedor, // Rol no válido para crear órdenes
                reputacion_como_comprador: 0,
                reputacion_como_vendedor: 0,
            };
            contrato.usuarios.insert(caller, &usuario);

            // Primero, publica un producto para poder comprarlo
            let _ = contrato.publicar_producto(
                "Tablet".to_string(),
                "Una tablet versátil".to_string(),
                1500,
                7,
                "Tecnología".to_string(),
            );

            let resultado = contrato.crear_orden(0, 1);

            assert!(matches!(resultado, Err(SistemaError::NoEsRolCorrecto)));
        }

        #[ink::test]
        fn crear_orden_con_rol_ambos_ok() {
            let mut contrato = setup_contract_con_vendedor();

            // Publica un producto
            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            // Cambia el caller a un usuario con rol Ambos
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let carlos = cuentas.charlie;
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            let _ = contrato.registrar_usuario(RolUsuario::Ambos);

            // Debería poder crear una orden exitosamente
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
        }

        #[ink::test]
        fn crear_orden_cantidad_insuficiente_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Primero, publica un producto con cantidad insuficiente
            let _ = contrato.publicar_producto(
                "Smartwatch".to_string(),
                "Un smartwatch elegante".to_string(),
                500,
                2, // Solo hay 2 disponibles
                "Tecnología".to_string(),
            );

            // Cambia el caller a un usuario comprador y regístralo
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Intenta crear una orden de compra de 3 unidades
            let resultado = contrato.crear_orden(0, 3); // Compra 3 unidades

            assert!(matches!(resultado, Err(SistemaError::StockInsuficiente)));
        }

        #[ink::test]
        fn crear_orden_cantidad_cero_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Publica un producto
            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            // Cambia el caller a un usuario comprador y regístralo
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Intenta crear una orden con cantidad 0
            let resultado = contrato.crear_orden(0, 0);

            assert!(matches!(resultado, Err(SistemaError::CantidadInsuficiente)));
        }

        #[ink::test]
        fn crear_orden_descuenta_stock() {
            let mut contrato = setup_contract_con_vendedor();

            // Primero, publica un producto con cantidad suficiente
            let _ = contrato.publicar_producto(
                "Auriculares".to_string(),
                "Auriculares inalámbricos".to_string(),
                800,
                10, // 10 disponibles
                "Tecnología".to_string(),
            );

            // Cambia el caller a un usuario comprador y regístralo
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Crea una orden de compra
            let resultado = contrato.crear_orden(0, 3); // Compra 3 unidades

            assert!(resultado.is_ok());
            assert_eq!(contrato.obtener_cantidad_ordenes(), 1);

            // Verifica que el stock se haya descontado correctamente
            let producto = contrato.productos.get(0).unwrap();
            assert_eq!(producto.cantidad, 7); // Debería quedar 7 después de la compra
        }

        

        // --- Gestión de órdenes ---
        #[ink::test]
        fn marcar_orden_como_enviada_ok() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);

            let resultado = contrato.marcar_orden_como_enviada(orden_id);
            assert!(resultado.is_ok());

            let orden = contrato.ordenes.get(orden_id).unwrap();
            assert_eq!(orden.estado, EstadoOrden::Enviada);
        }

        // --- Errores y validaciones ---
        #[ink::test]
        fn marcar_orden_como_enviada_usuario_no_registrado_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Publica un producto
            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            // Cambia el caller a un usuario comprador y regístralo
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Crea una orden
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            // Cambia el caller a un usuario NO registrado
            let usuario_no_registrado = AccountId::from([0x99; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(usuario_no_registrado);

            // Intenta marcar la orden como enviada
            let resultado = contrato.marcar_orden_como_enviada(orden_id);
            assert!(matches!(resultado, Err(SistemaError::UsuarioNoRegistrado)));
        }

        #[ink::test]
        fn marcar_como_recibida_usuario_no_registrado_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Publica un producto
            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            // Cambia el caller a un usuario comprador y regístralo
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Crea una orden
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            // Cambia el caller a un usuario NO registrado
            let usuario_no_registrado = AccountId::from([0x99; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(usuario_no_registrado);

            // Intenta marcar la orden como recibida
            let resultado = contrato.marcar_como_recibida(orden_id);
            assert!(matches!(resultado, Err(SistemaError::UsuarioNoRegistrado)));
        }

        #[ink::test]
        fn marcar_orden_como_enviada_usuario_no_es_vendedor_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Publica un producto
            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            // Cambia el caller a un usuario comprador y regístralo
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Crea una orden
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            // Cambia el caller a otro usuario registrado que NO es el vendedor
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let carlos = cuentas.charlie;
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            let _ = contrato.registrar_usuario(RolUsuario::Vendedor);

            // Intenta marcar la orden como enviada (no debería poder porque no es el vendedor de esta orden)
            let resultado = contrato.marcar_orden_como_enviada(orden_id);
            assert!(matches!(resultado, Err(SistemaError::NoEsRolCorrecto)));
        }

        #[ink::test]
        fn marcar_orden_como_enviada_orden_inexistente_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Intenta marcar una orden inexistente como enviada
            let resultado = contrato.marcar_orden_como_enviada(999);
            assert!(matches!(resultado, Err(SistemaError::OrdenNoExiste)));
        }

        #[ink::test]
        fn marcar_como_recibida_orden_inexistente_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Intenta marcar una orden inexistente como recibida
            let resultado = contrato.marcar_como_recibida(999);
            assert!(matches!(resultado, Err(SistemaError::OrdenNoExiste)));
        }

        #[ink::test]
        fn marcar_como_recibida_ok() {
            let mut contrato = setup_contract_con_vendedor();

            // Publica un producto
            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            // Cambia el caller a un usuario comprador y regístralo
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Crea una orden
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            // Cambia el caller de vuelta al vendedor para marcar como enviada
            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);

            // Marca la orden como enviada
            let resultado = contrato.marcar_orden_como_enviada(orden_id);
            assert!(resultado.is_ok());

            // Cambia el caller de vuelta al comprador para marcar como recibida
            test::set_caller::<ink::env::DefaultEnvironment>(juan);

            // Marca la orden como recibida (debe ser exitoso)
            let resultado = contrato.marcar_como_recibida(orden_id);
            assert!(resultado.is_ok());

            // Verifica que el estado cambió a Recibida
            let orden = contrato.ordenes.get(orden_id).unwrap();
            assert_eq!(orden.estado, EstadoOrden::Recibida);
        }

        #[ink::test]
        fn marcar_como_recibida_estado_pendiente_falla() {
            let mut contrato = setup_contract_con_vendedor();

            // Publica un producto
            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            // Cambia el caller a un usuario comprador y regístralo
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Crea una orden (estado inicial: Pendiente)
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            // Verifica que la orden está en estado Pendiente
            let orden = contrato.ordenes.get(orden_id).unwrap();
            assert_eq!(orden.estado, EstadoOrden::Pendiente);

            // Intenta marcar la orden como recibida directamente desde Pendiente (debe fallar)
            let resultado = contrato.marcar_como_recibida(orden_id);
            assert!(matches!(resultado, Err(SistemaError::EstadoInvalido)));
        }

        // --- Calificaciones ---
        #[ink::test]
        fn calificar_vendedor_ok() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let _ = contrato.marcar_orden_como_enviada(orden_id);

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.marcar_como_recibida(orden_id);

            let resultado = contrato.calificar_vendedor(orden_id, 5);
            assert!(resultado.is_ok());

            let vendor = contrato.obtener_usuario(vendedor).unwrap();
            assert_eq!(vendor.reputacion_como_vendedor, 5);
        }

        #[ink::test]
        fn calificar_vendedor_calificacion_invalida_falla() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let _ = contrato.marcar_orden_como_enviada(orden_id);

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.marcar_como_recibida(orden_id);

            let resultado = contrato.calificar_vendedor(orden_id, 6);
            assert!(matches!(resultado, Err(SistemaError::CalificacionInvalida)));
        }

        #[ink::test]
        fn calificar_vendedor_dos_veces_falla() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let _ = contrato.marcar_orden_como_enviada(orden_id);

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.marcar_como_recibida(orden_id);

            let _ = contrato.calificar_vendedor(orden_id, 5);
            let resultado = contrato.calificar_vendedor(orden_id, 3);
            assert!(matches!(resultado, Err(SistemaError::YaCalificado)));
        }

        #[ink::test]
        fn calificar_comprador_ok() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let _ = contrato.marcar_orden_como_enviada(orden_id);

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.marcar_como_recibida(orden_id);

            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let resultado = contrato.calificar_comprador(orden_id, 4);
            assert!(resultado.is_ok());

            let comprador = contrato.obtener_usuario(juan).unwrap();
            assert_eq!(comprador.reputacion_como_comprador, 4);
        }

        #[ink::test]
        fn calificar_comprador_dos_veces_falla() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let _ = contrato.marcar_orden_como_enviada(orden_id);

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.marcar_como_recibida(orden_id);

            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let _ = contrato.calificar_comprador(orden_id, 4);
            let resultado = contrato.calificar_comprador(orden_id, 3);
            assert!(matches!(resultado, Err(SistemaError::YaCalificado)));
        }

        // --- Errores y validaciones ---
        #[ink::test]
        fn marcar_como_recibida_usuario_no_es_comprador_falla() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "Producto Test".to_string(),
                "Descripción Test".to_string(),
                1000,
                10,
                "Test".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
            let orden_id = resultado.unwrap();

            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let resultado = contrato.marcar_orden_como_enviada(orden_id);
            assert!(resultado.is_ok());

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let carlos = cuentas.charlie;
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let resultado = contrato.marcar_como_recibida(orden_id);
            assert!(matches!(resultado, Err(SistemaError::NoEsRolCorrecto)));
        }

                // --- Flujos completos de compra y calificación ---
        #[ink::test]
        fn flujo_completo_compra_y_calificacion_ok() {
            let mut contrato = setup_contract_con_vendedor();

            // Vendedor publica producto
            let _ = contrato.publicar_producto(
                "Monitor".to_string(),
                "Monitor 4K".to_string(),
                3000,
                5,
                "Electrónica".to_string(),
            );

            // Comprador se registra y compra
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);
            let orden_id = contrato.crear_orden(0, 1).unwrap();

            // Vendedor marca como enviada
            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let _ = contrato.marcar_orden_como_enviada(orden_id);

            // Comprador marca como recibida
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.marcar_como_recibida(orden_id);

            // Comprador califica vendedor
            let _ = contrato.calificar_vendedor(orden_id, 5);
            let vendor = contrato.obtener_usuario(vendedor).unwrap();
            assert_eq!(vendor.reputacion_como_vendedor, 5);

            // Vendedor califica comprador
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let _ = contrato.calificar_comprador(orden_id, 4);
            let comprador = contrato.obtener_usuario(juan).unwrap();
            assert_eq!(comprador.reputacion_como_comprador, 4);
        }

        #[ink::test]
        fn multiples_ordenes_descuentan_stock_correctamente() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "Mouse".to_string(),
                "Mouse inalámbrico".to_string(),
                500,
                10,
                "Periféricos".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Primera orden: 3 unidades
            let _ = contrato.crear_orden(0, 3);
            assert_eq!(contrato.obtener_cantidad_ordenes(), 1);

            // Segunda orden: 2 unidades
            let _ = contrato.crear_orden(0, 2);
            assert_eq!(contrato.productos.get(0).unwrap().cantidad, 5);

            // Tercera orden: 5 unidades
            let _ = contrato.crear_orden(0, 5);
            assert_eq!(contrato.productos.get(0).unwrap().cantidad, 0);

            // Cuarta orden debería fallar por stock insuficiente
            let resultado = contrato.crear_orden(0, 1);
            assert!(matches!(resultado, Err(SistemaError::StockInsuficiente)));
        }

        #[ink::test]
        fn calificacion_acumulada_multiples_vendedores() {
            let mut contrato = setup_contract_con_vendedor();

            // Primer vendedor publica
            let _ = contrato.publicar_producto(
                "Producto1".to_string(),
                "Desc1".to_string(),
                1000,
                5,
                "Cat1".to_string(),
            );

            let vendedor1 = AccountId::from([0x10; 32]);
            let vendedor2 = AccountId::from([0x20; 32]);

            // Segundo vendedor se registra y publica
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor2);
            let _ = contrato.registrar_usuario(RolUsuario::Vendedor);
            let _ = contrato.publicar_producto(
                "Producto2".to_string(),
                "Desc2".to_string(),
                2000,
                5,
                "Cat2".to_string(),
            );

            // Comprador compra de ambos
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let orden1 = contrato.crear_orden(0, 1).unwrap();
            let orden2 = contrato.crear_orden(1, 1).unwrap();

            // Completar ambas órdenes
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor1);
            let _ = contrato.marcar_orden_como_enviada(orden1);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor2);
            let _ = contrato.marcar_orden_como_enviada(orden2);

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.marcar_como_recibida(orden1);
            let _ = contrato.marcar_como_recibida(orden2);

            // Calificar ambos
            let _ = contrato.calificar_vendedor(orden1, 5);
            let _ = contrato.calificar_vendedor(orden2, 3);

            let v1 = contrato.obtener_usuario(vendedor1).unwrap();
            let v2 = contrato.obtener_usuario(vendedor2).unwrap();
            assert_eq!(v1.reputacion_como_vendedor, 5);
            assert_eq!(v2.reputacion_como_vendedor, 3);
        }

        #[ink::test]
        fn cambio_rol_ambos_a_comprador_puede_comprar() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "Test".to_string(),
                "Test".to_string(),
                100,
                5,
                "Test".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let carlos = cuentas.charlie;
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            let _ = contrato.registrar_usuario(RolUsuario::Ambos);

            // Cambiar a Comprador
            let _ = contrato.modificar_rol_usuario(RolUsuario::Comprador);

            // Debe poder comprar (aunque sea Comprador)
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_ok());
        }

        #[ink::test]
        fn vendedor_no_puede_cambiar_a_ambos_directamente() {
            let mut contrato = Marketplace::nuevo();

            let caller = AccountId::from([0x07; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(caller);

            let _ = contrato.registrar_usuario(RolUsuario::Vendedor);

            // Intenta cambiar de Vendedor a Ambos (no está permitido)
            let resultado = contrato.modificar_rol_usuario(RolUsuario::Ambos);
            assert!(matches!(resultado, Err(SistemaError::NoEsRolCorrecto)));
        }

        #[ink::test]
        fn comprador_no_puede_cambiar_a_ambos_directamente() {
            let mut contrato = Marketplace::nuevo();

            let caller = AccountId::from([0x08; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(caller);

            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            // Intenta cambiar de Comprador a Ambos (no está permitido)
            let resultado = contrato.modificar_rol_usuario(RolUsuario::Ambos);
            assert!(matches!(resultado, Err(SistemaError::NoEsRolCorrecto)));
        }

        #[ink::test]
        fn crear_multiples_productos_incrementa_ids() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "P1".to_string(),
                "D1".to_string(),
                100,
                5,
                "C1".to_string(),
            );
            let _ = contrato.publicar_producto(
                "P2".to_string(),
                "D2".to_string(),
                200,
                3,
                "C2".to_string(),
            );
            let _ = contrato.publicar_producto(
                "P3".to_string(),
                "D3".to_string(),
                300,
                2,
                "C3".to_string(),
            );

            assert_eq!(contrato.productos.get(0).unwrap().id, 0);
            assert_eq!(contrato.productos.get(1).unwrap().id, 1);
            assert_eq!(contrato.productos.get(2).unwrap().id, 2);
        }

        #[ink::test]
        fn calificar_con_valor_cero_falla() {
            let mut contrato = setup_contract_con_vendedor();

            let _ = contrato.publicar_producto(
                "P".to_string(),
                "D".to_string(),
                100,
                5,
                "C".to_string(),
            );

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let orden_id = contrato.crear_orden(0, 1).unwrap();

            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            let _ = contrato.marcar_orden_como_enviada(orden_id);

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.marcar_como_recibida(orden_id);

            let resultado = contrato.calificar_vendedor(orden_id, 0);
            assert!(matches!(resultado, Err(SistemaError::CalificacionInvalida)));
        }

        #[ink::test]
        fn usuario_ambos_puede_listar_sus_productos() {
            let mut contrato = Marketplace::nuevo();

            let caller = AccountId::from([0x09; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(caller);

            let _ = contrato.registrar_usuario(RolUsuario::Ambos);

            let _ = contrato.publicar_producto(
                "P1".to_string(),
                "D1".to_string(),
                100,
                5,
                "C1".to_string(),
            );

            let productos = contrato.listar_mis_productos().unwrap();
            assert_eq!(productos.len(), 1);
            assert_eq!(productos[0].nombre, "P1");
        }

        #[ink::test]
        fn orden_no_existe_al_calificar() {
            let mut contrato = setup_contract_con_vendedor();

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            let _ = contrato.registrar_usuario(RolUsuario::Comprador);

            let resultado = contrato.calificar_vendedor(999, 5);
            assert!(matches!(resultado, Err(SistemaError::OrdenNoExiste)));
        }

        #[ink::test]
        fn calificar_comprador_orden_no_existe() {
            let mut contrato = setup_contract_con_vendedor();

            let resultado = contrato.calificar_comprador(999, 5);
            assert!(matches!(resultado, Err(SistemaError::OrdenNoExiste)));
        }

        // hay ejemplos en la documentación (doctests) que rustdoc intenta compilar/ejecutar y pueden fallar si no son autocontenidos.

        #[ink::test]
        fn producto_publicado_emite_evento() {
            let mut contrato = setup_contract_con_vendedor();
            let antes = test::recorded_events().count();
            contrato
                .publicar_producto(
                    "Camara".to_string(),
                    "Camara HD".to_string(),
                    1200,
                    3,
                    "Fotografia".to_string(),
                )
                .unwrap();
            let despues = test::recorded_events().count();
            assert!(despues > antes, "Se debe emitir al menos un evento al publicar");
        }

        #[ink::test]
        fn solicitar_cancelacion_mutua_devuelve_stock() {
            let mut contrato = setup_contract_con_vendedor();

            // Publicar producto con cantidad 5
            contrato
                .publicar_producto(
                    "Libro".to_string(),
                    "Libro técnico".to_string(),
                    200,
                    5,
                    "Libros".to_string(),
                )
                .unwrap();

            // Comprador registra y compra 2 unidades
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            let orden_id = contrato.crear_orden(0, 2).unwrap();

            // Stock debería quedar en 3
            assert_eq!(contrato.productos.get(0).unwrap().cantidad, 3);

            // Comprador solicita cancelación
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.solicitar_cancelacion_orden(orden_id).unwrap();

            // Vendedor acepta cancelación
            let vendedor = AccountId::from([0x10; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(vendedor);
            contrato.solicitar_cancelacion_orden(orden_id).unwrap();

            // Orden debe estar cancelada y stock restaurado a 5
            let orden = contrato.ordenes.get(orden_id).unwrap();
            assert_eq!(orden.estado, EstadoOrden::Cancelada);
            let producto = contrato.productos.get(0).unwrap();
            assert_eq!(producto.cantidad, 5);
        }

        #[ink::test]
        fn solicitar_cancelacion_unilateral_no_devuelve_stock() {
            let mut contrato = setup_contract_con_vendedor();

            contrato
                .publicar_producto(
                    "Teclado".to_string(),
                    "Mecánico".to_string(),
                    800,
                    4,
                    "Periféricos".to_string(),
                )
                .unwrap();

            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            let orden_id = contrato.crear_orden(0, 1).unwrap();

            // Comprador solicita cancelación
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.solicitar_cancelacion_orden(orden_id).unwrap();

            // Orden no debe estar cancelada hasta que el vendedor acepte
            let orden = contrato.ordenes.get(orden_id).unwrap();
            assert_eq!(orden.estado, EstadoOrden::Pendiente);
            // Stock no debe haber sido restaurado aún
            assert_eq!(contrato.productos.get(0).unwrap().cantidad, 3);
        }

        #[ink::test]
        fn listar_productos_por_vendedor_message_funciona() {
            let mut contrato = setup_contract_con_vendedor();

            contrato
                .publicar_producto(
                    "Papel".to_string(),
                    "A4 500 hojas".to_string(),
                    50,
                    10,
                    "Oficina".to_string(),
                )
                .unwrap();

            let vendedor = AccountId::from([0x10; 32]);
            let lista = contrato.listar_productos_por_vendedor(vendedor).unwrap();
            assert_eq!(lista.len(), 1);
            assert_eq!(lista[0].vendedor, vendedor);
            assert_eq!(lista[0].nombre, "Papel");
        }

        #[ink::test]
        fn evento_producto_publicado_aumenta_contador() {
            let mut contrato = setup_contract_con_vendedor();
            let inicial = test::recorded_events().collect::<Vec<_>>().len();
            contrato
                .publicar_producto(
                    "SSD".to_string(),
                    "1TB NVMe".to_string(),
                    15000,
                    2,
                    "Almacenamiento".to_string(),
                )
                .unwrap();
            let final_len = test::recorded_events().collect::<Vec<_>>().len();
            assert!(final_len >= inicial + 1);
        }

        #[ink::test]
        fn modificar_rol_emite_evento_y_actualiza_storage() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            test::set_caller::<ink::env::DefaultEnvironment>(maria);

            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            contrato.modificar_rol_usuario(RolUsuario::Vendedor).unwrap();

            // Verifica storage
            let u = contrato.obtener_usuario(maria).unwrap();
            assert_eq!(u.rol, RolUsuario::Vendedor);

            // Verifica que se emitió evento
            let eventos = test::recorded_events().collect::<Vec<_>>();
            assert!(!eventos.is_empty());
        }

        // =====================================================================
        // TESTS ADICIONALES PARA ALCANZAR 85%+ COBERTURA
        // =====================================================================

        #[ink::test]
        fn obtener_todos_los_productos_devuelve_correcto() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P1".to_string(), "Desc1".to_string(), 100, 5, "Cat1".to_string()).unwrap();
            contrato.publicar_producto("P2".to_string(), "Desc2".to_string(), 200, 3, "Cat2".to_string()).unwrap();
            
            let productos = contrato.obtener_todos_los_productos();
            assert_eq!(productos.len(), 2);
        }

        #[ink::test]
        fn obtener_todas_las_ordenes_devuelve_correcto() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P1".to_string(), "Desc".to_string(), 100, 5, "Cat".to_string()).unwrap();
            
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            
            contrato.crear_orden(0, 2).unwrap();
            contrato.crear_orden(0, 1).unwrap();
            
            let ordenes = contrato.obtener_todas_las_ordenes();
            assert_eq!(ordenes.len(), 2);
        }

        #[ink::test]
        fn obtener_todos_los_usuarios_devuelve_correcto() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;
            let carlos = cuentas.charlie;
            
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            contrato.registrar_usuario(RolUsuario::Ambos).unwrap();
            
            let usuarios = contrato.obtener_todos_los_usuarios();
            assert_eq!(usuarios.len(), 3);
        }

        #[ink::test]
        fn esta_registrado_verifica_correctamente() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;
            
            assert!(!contrato.esta_registrado(maria));
            
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            
            assert!(contrato.esta_registrado(maria));
            assert!(!contrato.esta_registrado(juan));
        }

        #[ink::test]
        fn obtener_usuario_retorna_option_correcta() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            
            assert_eq!(contrato.obtener_usuario(maria), None);
            
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            
            let usuario_opt = contrato.obtener_usuario(maria);
            assert!(usuario_opt.is_some());
            let usuario = usuario_opt.unwrap();
            assert_eq!(usuario.rol, RolUsuario::Comprador);
        }

        #[ink::test]
        fn cantidad_cero_en_crear_orden_falla() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
            
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            
            let resultado = contrato.crear_orden(0, 0);
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn no_comprador_no_puede_crear_orden() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
            
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            
            // Vendedor no puede comprar
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn crear_orden_sin_registrar_falla() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
            
            let usuario_no_registrado = AccountId::from([0xEE; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(usuario_no_registrado);
            
            let resultado = contrato.crear_orden(0, 1);
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn marcar_enviada_usuario_no_registrado_falla() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
            
            let usuario_no_registrado = AccountId::from([0xDD; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(usuario_no_registrado);
            
            let resultado = contrato.marcar_orden_como_enviada(0);
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn marcar_recibida_usuario_no_registrado_falla() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
            
            let usuario_no_registrado = AccountId::from([0xCC; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(usuario_no_registrado);
            
            let resultado = contrato.marcar_como_recibida(0);
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn calificar_vendedor_usuario_no_registrado_falla() {
            let resultado_calificar = {
                let mut contrato = setup_contract_con_vendedor();
                contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
                
                let usuario_no_registrado = AccountId::from([0xBB; 32]);
                test::set_caller::<ink::env::DefaultEnvironment>(usuario_no_registrado);
                
                contrato.calificar_vendedor(0, 5)
            };
            assert!(resultado_calificar.is_err());
        }

        #[ink::test]
        fn calificar_comprador_usuario_no_registrado_falla() {
            let resultado_calificar = {
                let mut contrato = setup_contract_con_vendedor();
                contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
                
                let usuario_no_registrado = AccountId::from([0xAA; 32]);
                test::set_caller::<ink::env::DefaultEnvironment>(usuario_no_registrado);
                
                contrato.calificar_comprador(0, 5)
            };
            assert!(resultado_calificar.is_err());
        }

        #[ink::test]
        fn comprador_no_puede_publicar_producto() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            
            let resultado = contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string());
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn no_registrado_no_puede_publicar() {
            let mut contrato = Marketplace::nuevo();
            let usuario_no_registrado = AccountId::from([0x99; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(usuario_no_registrado);
            
            let resultado = contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string());
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn cantidad_producto_cero_falla() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            
            let resultado = contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 0, "C".to_string());
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn marcar_enviada_no_vendedor_falla() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
            
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            contrato.crear_orden(0, 1).unwrap();
            
            // Comprador intenta marcar como enviada (solo vendedor puede)
            let resultado = contrato.marcar_orden_como_enviada(0);
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn marcar_recibida_no_comprador_falla() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
            
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            
            // Vendedor intenta marcar como recibida (solo comprador puede)
            let resultado = contrato.marcar_como_recibida(0);
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn marcar_recibida_estado_incorrecto_falla() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
            
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            let orden_id = contrato.crear_orden(0, 1).unwrap();
            
            // Orden en estado Pendiente no puede pasar a Recibida directamente
            let resultado = contrato.marcar_como_recibida(orden_id);
            assert!(resultado.is_err());
        }

        #[ink::test]
        fn rol_ambos_puede_hacer_todas_operaciones() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Ambos).unwrap();
            
            // Puede publicar producto (como vendedor)
            let resultado_pub = contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string());
            assert!(resultado_pub.is_ok());
            
            // Puede crear orden (como comprador)
            let resultado_orden = contrato.crear_orden(0, 1);
            assert!(resultado_orden.is_ok());
        }

        #[ink::test]
        fn calificar_comprador_orden_pendiente_falla() {
            let mut contrato = setup_contract_con_vendedor();
            contrato.publicar_producto("P".to_string(), "D".to_string(), 100, 5, "C".to_string()).unwrap();
            
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let juan = cuentas.bob;
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            let orden_id = contrato.crear_orden(0, 1).unwrap();
            
            let maria = cuentas.alice;
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            // Orden en Pendiente no puede ser calificada todavía
            let resultado = contrato.calificar_comprador(orden_id, 5);
            assert!(resultado.is_err());
        }

        // ===================== E2E TESTS - CICLOS COMPLETOS DE NEGOCIO =====================

        #[ink::test]
        fn e2e_compra_completa_sin_cancelacion() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;

            // 1. Maria (vendedor) se registra
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();

            // 2. Maria publica un producto
            contrato.publicar_producto("Laptop".to_string(), "Gaming laptop".to_string(), 1000, 10, "Electrónica".to_string()).unwrap();

            // 3. Juan (comprador) se registra
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();

            // 4. Juan crea una orden
            let orden_id = contrato.crear_orden(0, 2).unwrap();
            assert_eq!(orden_id, 0);

            // 5. Maria marca la orden como enviada
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.marcar_orden_como_enviada(orden_id).unwrap();

            // 6. Juan marca la orden como recibida
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.marcar_como_recibida(orden_id).unwrap();

            // 7. Juan califica a Maria (vendedor)
            contrato.calificar_vendedor(orden_id, 5).unwrap();

            // 8. Maria califica a Juan (comprador)
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.calificar_comprador(orden_id, 5).unwrap();

            // 9. Verificar reputación
            let maria_user = contrato.obtener_usuario(maria).unwrap();
            assert_eq!(maria_user.reputacion_como_vendedor, 5);
            
            let juan_user = contrato.obtener_usuario(juan).unwrap();
            assert_eq!(juan_user.reputacion_como_comprador, 5);
        }

        #[ink::test]
        fn e2e_compra_con_cancelacion_aceptada() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;

            // Setup: vendedor y producto
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            contrato.publicar_producto("Teclado".to_string(), "Mecánico".to_string(), 150, 20, "Periféricos".to_string()).unwrap();

            // Setup: comprador y orden
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            let orden_id = contrato.crear_orden(0, 5).unwrap();

            // Juan solicita cancelación
            contrato.solicitar_cancelacion_orden(orden_id).unwrap();
            let orden = contrato.obtener_todas_las_ordenes().iter().find(|o| o.id == orden_id).unwrap().clone();
            assert!(orden.comprador_solicita_cancelacion);
            assert_eq!(orden.estado, EstadoOrden::Pendiente);

            // Maria acepta la cancelación (no hace nada, asume aceptación automática)
            // En este caso verificamos que la orden fue marcada para cancelación
            assert!(orden.comprador_solicita_cancelacion);
        }

        #[ink::test]
        fn e2e_multiples_productos_mismo_vendedor() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;

            // Maria (vendedor) publica 3 productos
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            
            contrato.publicar_producto("Producto1".to_string(), "Desc1".to_string(), 100, 5, "Cat1".to_string()).unwrap();
            contrato.publicar_producto("Producto2".to_string(), "Desc2".to_string(), 200, 10, "Cat2".to_string()).unwrap();
            contrato.publicar_producto("Producto3".to_string(), "Desc3".to_string(), 300, 15, "Cat3".to_string()).unwrap();

            // Verificar que se publicaron correctamente
            let productos_maria = contrato.listar_mis_productos().unwrap();
            assert_eq!(productos_maria.len(), 3);

            // Juan compra de todos
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();

            for i in 0..3 {
                let orden_id = contrato.crear_orden(i as u32, 2).unwrap();
                
                test::set_caller::<ink::env::DefaultEnvironment>(maria);
                contrato.marcar_orden_como_enviada(orden_id).unwrap();
                
                test::set_caller::<ink::env::DefaultEnvironment>(juan);
                contrato.marcar_como_recibida(orden_id).unwrap();
                contrato.calificar_vendedor(orden_id, 5).unwrap();
            }

            // Verificar reputación de Maria después de 3 compras
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            let maria_user = contrato.obtener_usuario(maria).unwrap();
            assert_eq!(maria_user.reputacion_como_vendedor, 15);
        }

        #[ink::test]
        fn e2e_rol_ambos_ciclo_completo() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;

            // Maria con rol Ambos
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Ambos).unwrap();

            // Maria publica producto (como vendedor)
            contrato.publicar_producto("Libro".to_string(), "Rust Programming".to_string(), 50, 20, "Libros".to_string()).unwrap();

            // Juan también con rol Ambos
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Ambos).unwrap();

            // Juan compra de Maria (como comprador)
            let orden_id_1 = contrato.crear_orden(0, 1).unwrap();

            // Flujo completo orden 1
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.marcar_orden_como_enviada(orden_id_1).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.marcar_como_recibida(orden_id_1).unwrap();
            contrato.calificar_vendedor(orden_id_1, 5).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.calificar_comprador(orden_id_1, 5).unwrap();

            // Ahora Juan publica (como vendedor) e Maria compra (como comprador)
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.publicar_producto("Notebook".to_string(), "Java".to_string(), 45, 10, "Libros".to_string()).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            let orden_id_2 = contrato.crear_orden(1, 1).unwrap();

            // Flujo completo orden 2
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.marcar_orden_como_enviada(orden_id_2).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.marcar_como_recibida(orden_id_2).unwrap();
            contrato.calificar_vendedor(orden_id_2, 4).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.calificar_comprador(orden_id_2, 4).unwrap();

            // Verificar reputación mutua
            let maria_user = contrato.obtener_usuario(maria).unwrap();
            assert_eq!(maria_user.reputacion_como_vendedor, 5);
            assert_eq!(maria_user.reputacion_como_comprador, 4);

            let juan_user = contrato.obtener_usuario(juan).unwrap();
            assert_eq!(juan_user.reputacion_como_vendedor, 4);
            assert_eq!(juan_user.reputacion_como_comprador, 5);
        }

        #[ink::test]
        fn e2e_cambio_rol_y_nueva_actividad() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let carlos = cuentas.charlie;

            // Maria se registra como Ambos (puede vender y comprar)
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Ambos).unwrap();
            contrato.publicar_producto("Mouse".to_string(), "Inalámbrico".to_string(), 30, 50, "Periféricos".to_string()).unwrap();

            // Carlos se registra como comprador
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();

            // Carlos compra a Maria
            let orden_id = contrato.crear_orden(0, 2).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.marcar_orden_como_enviada(orden_id).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            contrato.marcar_como_recibida(orden_id).unwrap();
            contrato.calificar_vendedor(orden_id, 5).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.calificar_comprador(orden_id, 5).unwrap();

            // Carlos cambia su rol de Comprador a Vendedor
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            contrato.modificar_rol_usuario(RolUsuario::Vendedor).unwrap();

            // Carlos publica un producto
            contrato.publicar_producto("Monitor".to_string(), "4K".to_string(), 400, 5, "Monitores".to_string()).unwrap();

            // Verificar cambio de rol
            let carlos_user = contrato.obtener_usuario(carlos).unwrap();
            assert_eq!(carlos_user.rol, RolUsuario::Vendedor);

            // Maria compra de Carlos (Maria cambia a Comprador)
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            let orden_id_2 = contrato.crear_orden(1, 1).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            contrato.marcar_orden_como_enviada(orden_id_2).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.marcar_como_recibida(orden_id_2).unwrap();
            contrato.calificar_vendedor(orden_id_2, 4).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            contrato.calificar_comprador(orden_id_2, 5).unwrap();

            // Verificar reputación final de Carlos
            let carlos_final = contrato.obtener_usuario(carlos).unwrap();
            assert_eq!(carlos_final.reputacion_como_vendedor, 4);
        }

        #[ink::test]
        fn e2e_stock_management_flujo_completo() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;

            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            contrato.publicar_producto("USB".to_string(), "16GB".to_string(), 20, 5, "Memoria".to_string()).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();

            // Juan intenta comprar más del stock disponible
            let resultado = contrato.crear_orden(0, 10);
            assert!(resultado.is_err());

            // Juan compra 3 (stock reduce a 2)
            let _orden_id_1 = contrato.crear_orden(0, 3).unwrap();

            // Verificar stock disponible
            let productos = contrato.obtener_todos_los_productos();
            let usb = productos.iter().find(|p| p.id == 0).unwrap();
            assert_eq!(usb.cantidad, 2);

            // Juan crea otra orden con 2 (stock final 0)
            let _orden_id_2 = contrato.crear_orden(0, 2).unwrap();

            let productos_updated = contrato.obtener_todos_los_productos();
            let usb_updated = productos_updated.iter().find(|p| p.id == 0).unwrap();
            assert_eq!(usb_updated.cantidad, 0);

            // Juan intenta comprar más sin stock
            let resultado_sin_stock = contrato.crear_orden(0, 1);
            assert!(resultado_sin_stock.is_err());
        }

        #[ink::test]
        fn e2e_multiples_compradores_mismo_producto() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;
            let carlos = cuentas.charlie;

            // Maria vendedor
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            contrato.publicar_producto("Monitor".to_string(), "27 pulgadas".to_string(), 300, 100, "Electrónica".to_string()).unwrap();

            // Juan comprador
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            let orden_juan = contrato.crear_orden(0, 10).unwrap();

            // Carlos comprador
            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            let orden_carlos = contrato.crear_orden(0, 15).unwrap();

            // Dave comprador
            let dave = AccountId::from([0x04; 32]);
            test::set_caller::<ink::env::DefaultEnvironment>(dave);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();
            let orden_dave = contrato.crear_orden(0, 20).unwrap();

            // Maria procesa todos
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.marcar_orden_como_enviada(orden_juan).unwrap();
            contrato.marcar_orden_como_enviada(orden_carlos).unwrap();
            contrato.marcar_orden_como_enviada(orden_dave).unwrap();

            // Todos marcan recibido y califican
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.marcar_como_recibida(orden_juan).unwrap();
            contrato.calificar_vendedor(orden_juan, 5).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(carlos);
            contrato.marcar_como_recibida(orden_carlos).unwrap();
            contrato.calificar_vendedor(orden_carlos, 5).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(dave);
            contrato.marcar_como_recibida(orden_dave).unwrap();
            contrato.calificar_vendedor(orden_dave, 4).unwrap();

            // Maria califica a todos
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.calificar_comprador(orden_juan, 5).unwrap();
            contrato.calificar_comprador(orden_carlos, 5).unwrap();
            contrato.calificar_comprador(orden_dave, 5).unwrap();

            // Maria debe tener 14 puntos de reputación (5+5+4)
            let maria_user = contrato.obtener_usuario(maria).unwrap();
            assert_eq!(maria_user.reputacion_como_vendedor, 14);

            // Verificar que todas las órdenes pasaron por todos los estados
            let ordenes = contrato.obtener_todas_las_ordenes();
            let orden_juan_final = ordenes.iter().find(|o| o.id == orden_juan).unwrap();
            let orden_carlos_final = ordenes.iter().find(|o| o.id == orden_carlos).unwrap();
            let orden_dave_final = ordenes.iter().find(|o| o.id == orden_dave).unwrap();

            assert_eq!(orden_juan_final.estado, EstadoOrden::Recibida);
            assert_eq!(orden_carlos_final.estado, EstadoOrden::Recibida);
            assert_eq!(orden_dave_final.estado, EstadoOrden::Recibida);

            assert!(orden_juan_final.comprador_califico);
            assert!(orden_juan_final.vendedor_califico);
            assert!(orden_carlos_final.comprador_califico);
            assert!(orden_carlos_final.vendedor_califico);
            assert!(orden_dave_final.comprador_califico);
            assert!(orden_dave_final.vendedor_califico);
        }

        #[ink::test]
        fn e2e_error_recovery_flow() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;

            // Maria vendedor
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            contrato.publicar_producto("Producto".to_string(), "Desc".to_string(), 100, 10, "Cat".to_string()).unwrap();

            // Juan comprador
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();

            // Juan intenta operaciones inválidas
            assert!(contrato.marcar_orden_como_enviada(999).is_err());
            assert!(contrato.marcar_como_recibida(999).is_err());
            assert!(contrato.calificar_vendedor(999, 5).is_err());

            // Juan crea orden válida
            let orden_id = contrato.crear_orden(0, 2).unwrap();

            // Juan intenta calificar vendedor antes de que sea enviada
            assert!(contrato.calificar_vendedor(orden_id, 5).is_err());

            // Maria envía
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.marcar_orden_como_enviada(orden_id).unwrap();

            // Juan intenta calificar vendedor antes de que sea recibida
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            assert!(contrato.calificar_vendedor(orden_id, 5).is_err());

            // Juan marca recibido
            contrato.marcar_como_recibida(orden_id).unwrap();

            // Ahora sí puede calificar
            assert!(contrato.calificar_vendedor(orden_id, 5).is_ok());

            // Juan intenta calificar dos veces
            assert!(contrato.calificar_vendedor(orden_id, 3).is_err());
        }

        #[ink::test]
        fn e2e_reputacion_calificaciones_extremas() {
            let mut contrato = Marketplace::nuevo();
            let cuentas = test::default_accounts::<ink::env::DefaultEnvironment>();
            let maria = cuentas.alice;
            let juan = cuentas.bob;

            // Maria es vendedor, crea 5 productos
            test::set_caller::<ink::env::DefaultEnvironment>(maria);
            contrato.registrar_usuario(RolUsuario::Vendedor).unwrap();
            contrato.publicar_producto("P1".to_string(), "D1".to_string(), 50, 5, "C1".to_string()).unwrap();
            contrato.publicar_producto("P2".to_string(), "D2".to_string(), 60, 5, "C2".to_string()).unwrap();
            contrato.publicar_producto("P3".to_string(), "D3".to_string(), 70, 5, "C3".to_string()).unwrap();
            contrato.publicar_producto("P4".to_string(), "D4".to_string(), 80, 5, "C4".to_string()).unwrap();
            contrato.publicar_producto("P5".to_string(), "D5".to_string(), 90, 5, "C5".to_string()).unwrap();

            // Juan es comprador
            test::set_caller::<ink::env::DefaultEnvironment>(juan);
            contrato.registrar_usuario(RolUsuario::Comprador).unwrap();

            // 5 órdenes, todas con calificación 5
            for i in 0..5 {
                test::set_caller::<ink::env::DefaultEnvironment>(juan);
                let orden = contrato.crear_orden(i, 1).unwrap();

                test::set_caller::<ink::env::DefaultEnvironment>(maria);
                contrato.marcar_orden_como_enviada(orden).unwrap();

                test::set_caller::<ink::env::DefaultEnvironment>(juan);
                contrato.marcar_como_recibida(orden).unwrap();
                contrato.calificar_vendedor(orden, 5).unwrap();

                test::set_caller::<ink::env::DefaultEnvironment>(maria);
                contrato.calificar_comprador(orden, 5).unwrap();
            }

            // Maria debe tener 25 en reputacion de vendedor y 0 en comprador
            let maria_user = contrato.obtener_usuario(maria).unwrap();
            assert_eq!(maria_user.reputacion_como_vendedor, 25);
            assert_eq!(maria_user.reputacion_como_comprador, 0);

            // Juan debe tener 25 como comprador
            let juan_user = contrato.obtener_usuario(juan).unwrap();
            assert_eq!(juan_user.reputacion_como_comprador, 25);
        }
    } // <-- cierre del mod tests
} // <-- cierre del mod marketplace_principal
