use std::borrow::BorrowMut;

use tauri::async_runtime;

use super::{
    pesable::Pesable, producto::Producto, proveedor::Proveedor, valuable::{ValuableTrait, Valuable}, rubro::Rubro, Config, Venta, RelacionProdProv, lib::{leer_file, crear_file},
};
pub struct Sistema {
    configs: Config,
    productos: Vec<Valuable>,
    ventas: (Venta, Venta),
    proveedores: Vec<Proveedor>,
    path_productos: String,
    path_proveedores: String,
    path_relaciones: String,
    path_configs: String,
    relaciones: Vec<RelacionProdProv>,
    stash: Vec<Venta>,
    registro: Vec<Venta>,
}

impl<'a> Sistema {
    pub fn new() -> Sistema {
        let path_productos = String::from("Productos.json");
        let path_proveedores = String::from("Proveedores.json");
        let path_relaciones = String::from("Relaciones.json");
        let path_configs = String::from("Configs.json");
        let path_rubros = String::from("Rubros.json");
        let path_pesables = String::from("Pesables.json");
        let mut productos: Vec<Producto> = Vec::new();
        let mut rubros: Vec<Rubro> = Vec::new();
        let mut pesables: Vec<Pesable> = Vec::new();
        let stash = Vec::new();
        let registro = Vec::new();
        if let Err(e) = leer_file(&mut rubros, &path_rubros) {
            panic!("{}", e);
        }
        if let Err(e) = leer_file(&mut pesables, &path_pesables) {
            panic!("{}", e);
        }
        if let Err(e) = leer_file(&mut productos, &path_productos) {
            panic!("{}", e);
        }

        let mut rubros: Vec<Valuable> = rubros
            .iter()
            .map(|a| Valuable::Rub((0, a.to_owned())))
            .collect();
        let mut pesables: Vec<Valuable> = pesables
            .iter()
            .map(|a| Valuable::Pes((0.0, a.to_owned())))
            .collect();
        let mut productos: Vec<Valuable> = productos
            .iter()
            .map(|a| Valuable::Prod((0, a.to_owned())))
            .collect();
        productos.append(&mut pesables);
        productos.append(&mut rubros);

        let mut proveedores: Vec<Proveedor> = Vec::new();
        if let Err(e) = leer_file(&mut proveedores, &path_proveedores) {
            panic!("{}", e);
        }
        let mut relaciones = Vec::new();
        if let Err(e) = leer_file(&mut relaciones, &path_relaciones) {
            panic!("{}", e);
        }
        let mut configs = Vec::<Config>::new();
        if let Err(e) = leer_file(&mut configs, &path_configs) {
            panic!("{}", e);
        }
        if configs.len() == 0 {
            configs.push(Config::default());
            if let Err(e) = crear_file(&path_configs, &mut configs) {
                panic!("{}", e);
            }
        }
        Sistema {
            configs: configs[0].clone(),
            productos,
            ventas: (Venta::new(), Venta::new()),
            proveedores,
            path_productos,
            path_proveedores,
            path_relaciones,
            path_configs,
            relaciones,
            stash,
            registro,
        }
    }
    pub fn get_productos(&self) -> &Vec<Valuable> {
        &self.productos
    }
    pub fn get_productos_cloned(&self) -> Vec<Valuable> {
        self.productos.clone()
    }
    pub fn get_proveedores(&self) -> &Vec<Proveedor> {
        &self.proveedores
    }
    pub fn get_configs(&self) -> &Config {
        &self.configs
    }
    pub fn get_venta_mut(&mut self, pos: usize) -> &mut Venta {
        if pos == 1 {
            self.ventas.1.borrow_mut()
        } else {
            self.ventas.0.borrow_mut()
        }
    }
    pub fn agregar_pago(
        &mut self,
        medio_pago: String,
        monto: f64,
        pos: usize,
    ) -> Result<f64, String> {
        let error_msj = "error, hay solo dos posiciones para ventas".to_string();
        let res;
        match pos {
            0 => {
                if !medio_pago.eq("Efectivo")
                    && self.ventas.0.monto_pagado + monto > self.ventas.0.monto_total
                {
                    res = Err(format!(
                        "El monto no puede ser superior al resto con {medio_pago}"
                    ));
                } else {
                    res = Ok(self.ventas.0.agregar_pago(medio_pago, monto));
                }
            }
            1 => {
                if !medio_pago.eq("Efectivo")
                    && self.ventas.1.monto_pagado + monto > self.ventas.1.monto_total
                {
                    res = Err(format!(
                        "El monto no puede ser superior al resto con {medio_pago}"
                    ));
                } else {
                    res = Ok(self.ventas.1.agregar_pago(medio_pago, monto));
                }
            }
            _ => res = Err(error_msj),
        }
        if let Ok(a) = res {
            if a <= 0.0 {
                self.cerrar_venta(pos);
            }
        }
        res
    }
    pub fn set_configs(&mut self, configs: Config) {
        self.configs = configs;
        if let Err(e) = crear_file(&self.path_configs, &vec![&self.configs]) {
            panic!("{e}");
        }
    }
    pub fn imprimir(&self) {
        println!("Printed from rust");
    }
    fn proveedor_esta(&self, proveedor: &str) -> bool {
        let mut res = false;
        for i in &self.proveedores {
            if i.get_nombre().eq_ignore_ascii_case(proveedor) {
                res = true;
            }
        }
        res
    }
    pub fn agregar_producto(
        &mut self,
        proveedores: Vec<String>,
        codigos_prov: Vec<String>,
        producto: Producto,
    ) -> Result<(), String> {
        let mut res = Ok(());

        self.productos.push(Valuable::Prod((0, producto)));

        for i in 0..proveedores.len() {
            match codigos_prov[i].parse::<i64>() {
                Ok(a) => self.relaciones.push(RelacionProdProv::new(
                    self.productos.len() as i64 - 1,
                    i as i64,
                    Some(a),
                )),
                Err(_) => self.relaciones.push(RelacionProdProv::new(
                    self.productos.len() as i64 - 1,
                    i as i64,
                    None,
                )),
            };
        }
        let productos: Vec<Producto> = self
            .productos
            .iter()
            .map(|x| match x {
                Valuable::Prod(a) => Some(a.1.clone()),
                Valuable::Pes(_) => None,
                Valuable::Rub(_) => None,
            })
            .flatten()
            .collect();
        match crear_file(&self.path_productos, &productos) {
            Ok(_) => (),
            Err(e) => res = Err(e.to_string()),
        }
        match crear_file(&self.path_relaciones, &self.relaciones) {
            Ok(_) => (),
            Err(e) => res = Err(e.to_string()),
        }
        res
    }
    pub fn agregar_proveedor(&mut self, proveedor: &str, contacto: &str) -> Result<(), String> {
        let mut res = Ok(());
        if self.proveedor_esta(&proveedor) {
            res = Err("Proveedor existente".to_owned());
        } else {
            let prov;
            if contacto.len() > 0 {
                let contacto: String = contacto
                    .chars()
                    .filter(|x| -> bool { x.is_numeric() })
                    .collect();
                let contacto = match contacto.parse() {
                    Ok(a) => Some(a),
                    Err(_) => return Err("Error al convertir el numero".to_owned()),
                };
                prov = Proveedor::new(
                    self.proveedores.len() as i64,
                    proveedor.to_owned(),
                    contacto,
                );
            } else {
                prov = Proveedor::new(self.proveedores.len() as i64, proveedor.to_owned(), None);
            }
            if let Err(e) = async_runtime::block_on(prov.save()) {
                return Err(e.to_string());
            }
            self.proveedores.push(prov);
            if let Err(e) = crear_file(&self.path_proveedores, &self.proveedores) {
                res = Err(e.to_string());
            }
        }
        res
    }
    fn get_producto(&mut self, id: i64) -> Result<Valuable, String> {
        let mut res = Err("Producto no encontrado".to_string());
        for p in &self.productos {
            match p {
                Valuable::Pes(a) => {
                    if a.1.id == id {
                        res = Ok(p.clone());
                    }
                }
                Valuable::Prod(a) => {
                    if a.1.id == id {
                        res = Ok(p.clone());
                    }
                }
                Valuable::Rub(a) => {
                    if a.1.id == id {
                        res = Ok(p.clone());
                    }
                }
            }
        }
        res
    }
    pub fn agregar_producto_a_venta(&mut self, id: i64, pos: usize) -> Result<Venta, String> {
        let res = self
            .get_producto(id)?
            .redondear(self.configs.politica_redondeo);
        let result;
        match pos {
            0 => {
                result = Ok(self
                    .ventas
                    .0
                    .agregar_producto(res, self.configs.politica_redondeo))
            }
            1 => {
                result = Ok(self
                    .ventas
                    .1
                    .agregar_producto(res, self.configs.politica_redondeo))
            }
            _ => result = Err("Numero de venta incorrecto".to_string()),
        }

        result
    }
    pub fn descontar_producto_de_venta(&mut self, id: i64, pos: usize) -> Result<Venta, String> {
        let res = self.get_producto(id)?;
        let result;
        match pos {
            0 => {
                result = self
                    .ventas
                    .0
                    .restar_producto(res, self.configs.politica_redondeo);
            }
            1 => {
                result = self
                    .ventas
                    .1
                    .restar_producto(res, self.configs.politica_redondeo);
            }
            _ => result = Err("Numero de venta incorrecto".to_string()),
        }
        result
    }
    pub fn incrementar_producto_a_venta(&mut self, id: i64, pos: usize) -> Result<Venta, String> {
        let res = self.get_producto(id)?;
        let result;
        match pos {
            0 => {
                result = self
                    .ventas
                    .0
                    .incrementar_producto(res, self.configs.politica_redondeo);
            }
            1 => {
                result = self
                    .ventas
                    .1
                    .incrementar_producto(res, self.configs.politica_redondeo);
            }
            _ => result = Err("Numero de venta incorrecto".to_string()),
        }
        result
    }
    pub fn eliminar_producto_de_venta(&mut self, id: i64, pos: usize) -> Result<Venta, String> {
        let res = self.get_producto(id)?;
        let result;
        match pos {
            0 => {
                result = self
                    .ventas
                    .0
                    .eliminar_producto(res, self.configs.politica_redondeo);
            }
            1 => {
                result = self
                    .ventas
                    .1
                    .eliminar_producto(res, self.configs.politica_redondeo);
            }
            _ => result = Err("Numero de venta incorrecto".to_string()),
        }
        result
    }
    pub fn get_venta(&self, pos: usize) -> Venta {
        let res;
        if pos == 0 {
            res = self.ventas.0.clone();
        } else {
            res = self.ventas.1.clone();
        }
        res
    }
    pub fn filtrar_marca(&self, filtro: &str) -> Vec<String> {
        let iter = self.productos.iter();
        let mut res: Vec<String> = iter
            .filter_map(|x| match x {
                Valuable::Prod(a) => {
                    if a.1.marca.to_lowercase().contains(&filtro.to_lowercase()) {
                        Some(a.1.marca.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();
        res.sort();
        res.dedup();

        res
    }

    pub fn filtrar_tipo_producto(&self, filtro: &str) -> Vec<String> {
        let iter = self.productos.iter();
        let mut res: Vec<String> = iter
            .filter_map(|x| match x {
                Valuable::Prod(a) => {
                    if a.1
                        .tipo_producto
                        .to_lowercase()
                        .contains(&filtro.to_lowercase())
                    {
                        Some(a.1.tipo_producto.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();
        res.sort();
        res.dedup();
        res
    }
    fn cerrar_venta(&mut self, pos: usize) {
        match pos {
            0 => {
                self.registro.push(self.ventas.0.clone());
                self.ventas.0 = Venta::new();
            }
            1 => {
                self.registro.push(self.ventas.1.clone());
                self.ventas.1 = Venta::new();
            }
            _ => panic!("error, solo hay 2 posiciones para ventas"),
        };
    }
    pub fn stash_sale(&mut self, pos: usize) {
        match pos {
            0 => {
                self.stash.push(self.ventas.0.clone());
                self.ventas.0 = Venta::new();
            }
            1 => {
                self.stash.push(self.ventas.1.clone());
                self.ventas.1 = Venta::new();
            }
            _ => panic!("error, solo hay 2 posiciones para ventas"),
        };
    }
    pub fn unstash_sale(&mut self, pos: usize, index: usize) -> Result<(), String> {
        match pos {
            0 => {
                self.ventas.0 = self.stash.remove(index);
                Ok(())
            }
            1 => {
                self.stash.push(self.ventas.1.clone());
                self.ventas.1 = self.stash.remove(index);
                Ok(())
            }
            _ => Err("error, solo hay 2 posiciones para ventas".to_string()),
        }
    }
    pub fn get_stash(&self) -> Vec<Venta> {
        self.stash.clone()
    }
}
