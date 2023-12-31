use chrono::Utc;
use entity::producto::{self, ActiveModel, Model};
use entity::{codigo_barras, pesable, proveedor};
use sea_orm::prelude::DateTimeUtc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, Database, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, Set,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};

type Res<T> = std::result::Result<T, Box<dyn Error>>;
use std::error::Error;

use crate::mods::valuable::Valuable;

use super::error::{AppError, SizeSelecionError};
use super::pesable::Pesable;
use super::producto::Producto;
use super::proveedor::Proveedor;
use super::relacion_prod_prov::RelacionProdProv;
use super::rubro::Rubro;
use super::valuable::Presentacion;

pub fn crear_file<'a>(path: &str, escritura: &impl Serialize) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    let buf = serde_json::to_string_pretty(escritura)?;
    write!(f, "{}", format!("{}", buf))?;
    Ok(())
}

pub fn camalize(data: &str) -> String {
    let mut es = true;
    let mut datos = String::new();
    for i in 0..data.len() {
        if es {
            if data.chars().nth(i) == None {
                datos.push('Ñ');
            } else {
                datos.push(
                    data.chars()
                        .nth(i)
                        .unwrap()
                        .to_uppercase()
                        .to_string()
                        .chars()
                        .last()
                        .unwrap(),
                )
            }
        } else {
            if data.chars().nth(i) == None {
                datos.push('ñ');
            } else {
                datos.push(
                    data.chars()
                        .nth(i)
                        .unwrap()
                        .to_lowercase()
                        .to_string()
                        .chars()
                        .last()
                        .unwrap(),
                )
            }
        }

        if data.chars().nth(i).is_some() && data.chars().nth(i).unwrap() == ' ' {
            es = true;
        } else {
            es = false;
        }
    }

    // for (i, mut a) in iter.char_indices() {
    //     println!("llego");
    //     if es {
    //         if a == 'ñ' || a == 'Ñ' {
    //             println!("es {}", a);
    //             data.replace_range(i..i + 1, 'Ñ'.to_string().as_str());
    //             println!("reemplazado");
    //         } else {
    //             a.make_ascii_uppercase();
    //             data.replace_range(i..i + 1, a.to_string().as_str());
    //         }
    //     } else {
    //         if a == 'ñ' || a == 'Ñ' {
    //             println!("es {}", a);
    //             data.replace_range(i..i + 1, 'ñ'.to_string().as_str());
    //             println!("reemplazado");
    //         } else {
    //             a.make_ascii_lowercase();
    //             data.replace_range(i..i + 1, a.to_string().as_str());
    //         }
    //     }
    //     if a == ' ' {
    //         es = true;
    //     } else {
    //         es = false
    //     }
    // }
    datos
}

pub fn leer_file<T: DeserializeOwned + Clone + Serialize>(
    buf: &mut T,
    path: &str,
) -> std::io::Result<()> {
    let file2 = File::open(path.clone());
    let mut file2 = match file2 {
        Ok(file) => file,
        Err(_) => {
            let esc: Vec<String> = Vec::new();
            crear_file(path, &esc)?;
            File::open(path.clone())?
        }
    };

    let mut buf2 = String::new();
    file2.read_to_string(&mut buf2)?;
    match serde_json::from_str::<T>(&buf2.clone()) {
        Ok(a) => *buf = a.clone(),
        Err(e) => panic!("No se pudo porque {}", e),
    }
    Ok(())
}
pub fn get_updated_time_file(path: &str) -> Res<DateTimeUtc> {
    let a = fs::metadata(path)?.modified()?;
    let res = a.duration_since(std::time::SystemTime::UNIX_EPOCH)?;

    make_elapsed_to_date(res).ok_or_else(|| AppError::DateFormat.into())
}
pub async fn get_updated_time_db_producto(vec: Vec<Model>) -> DateTimeUtc {
    vec.iter()
        .max_by_key(|x| x.updated_at)
        .unwrap()
        .updated_at
        .and_utc()
}
pub async fn get_updated_time_db_pesable(vec: Vec<entity::pesable::Model>) -> DateTimeUtc {
    vec.iter()
        .max_by_key(|x| x.updated_at)
        .unwrap()
        .updated_at
        .and_utc()
}
pub async fn get_updated_time_db_rubro(vec: Vec<entity::rubro::Model>) -> DateTimeUtc {
    vec.iter()
        .max_by_key(|x| x.updated_at)
        .unwrap()
        .updated_at
        .and_utc()
}

fn make_elapsed_to_date(date: std::time::Duration) -> Option<DateTimeUtc> {
    let (sec, nsec) = (date.as_secs() as i64, date.subsec_nanos());
    DateTimeUtc::from_timestamp(sec, nsec)
}

// pub fn push(pr: Producto, path: &String) {
//     let mut prods = Vec::new();
//     if let Err(e) = leer_file(&mut prods, path) {
//         panic!("{}", e);
//     }
//     prods.push(pr);
//     match crear_file(&path, &prods) {
//         Ok(_) => (),
//         Err(e) => panic!("No se pudo pushear porque {}", e),
//     };
// }
pub async fn get_codigos_db_filtrado(db: &DatabaseConnection, id: i64) -> Res<Vec<i64>> {
    let a = entity::codigo_barras::Entity::find()
        .filter(Condition::all().add(entity::codigo_barras::Column::Producto.eq(id)))
        .all(db)
        .await?;
    Ok(a.iter().map(|x| x.codigo).collect())
}

pub async fn update_data_valuable(
    rubros_local: &mut Vec<Rubro>,
    pesables_local: &mut Vec<Pesable>,
    productos_local: &mut Vec<Producto>,
    path_rubros: &str,
    path_pesables: &str,
    path_productos: &str,
) -> Res<bool> {
    let mut prods: Vec<Valuable>;
    let mut hay_cambios_desde_db = false;

    let db = Database::connect("postgres://postgres:L33tsupa@localhost:5432/Tauri").await?;
    let aux = update_productos_from_db(productos_local, path_productos, &db).await?;
    prods = productos_local
        .iter()
        .map(|x| Valuable::Prod((0, x.clone())))
        .collect();
    if !hay_cambios_desde_db {
        hay_cambios_desde_db = aux;
    }
    let aux = update_pesables_from_db(pesables_local, path_pesables, &db).await?;
    prods.append(
        &mut pesables_local
            .iter()
            .map(|x| Valuable::Pes((0.0, x.clone())))
            .collect(),
    );
    if !hay_cambios_desde_db {
        hay_cambios_desde_db = aux;
    }
    let aux = update_rubros_from_db(rubros_local, path_rubros, &db).await?;
    prods.append(
        &mut rubros_local
            .iter()
            .map(|x| Valuable::Rub((0, x.clone())))
            .collect(),
    );
    if !hay_cambios_desde_db {
        hay_cambios_desde_db = aux;
    }

    println!("Cantidad de propductos: {}", prods.len());

    Ok(hay_cambios_desde_db)
}

pub async fn update_rubros_from_db(
    rubros_local: &mut Vec<Rubro>,
    path_rubros: &str,
    db: &DatabaseConnection,
) -> Res<bool> {
    let mut hay_cambios_desde_db = false;
    let rubros_db_model = entity::rubro::Entity::find().all(db).await?;
    let mut date_db = DateTimeUtc::MIN_UTC;
    if rubros_db_model.len() > 0 {
        date_db = get_updated_time_db_rubro(rubros_db_model.clone()).await;
    }
    let date_local = get_updated_time_file(path_rubros)?;
    if date_db > date_local {
        rubros_local.clear();
        println!("Ultimo actualizado: rubros de bases de datos");
        for i in 0..rubros_db_model.len() {
            rubros_local.push(map_model_rub(&rubros_db_model[i]));
        }
        hay_cambios_desde_db = true;
    } else if date_db == date_local {
        println!("Rubros sincronizados")
    } else {
        println!("Ultimo actualizado: rubros local");
    }
    Ok(hay_cambios_desde_db)
}
pub async fn update_pesables_from_db(
    pesables_local: &mut Vec<Pesable>,
    path_pesables: &str,
    db: &DatabaseConnection,
) -> Res<bool> {
    let mut hay_cambios_desde_db = false;
    let pesables_db_model = entity::pesable::Entity::find().all(db).await?;
    let mut date_db = DateTimeUtc::MIN_UTC;
    if pesables_db_model.len() > 0 {
        date_db = get_updated_time_db_pesable(pesables_db_model.clone()).await;
    }
    let date_local = get_updated_time_file(path_pesables)?;
    if date_db > date_local {
        pesables_local.clear();
        println!("Ultimo actualizado: pesables de bases de datos");
        for i in 0..pesables_db_model.len() {
            pesables_local.push(map_model_pes(&pesables_db_model[i]));
        }
        hay_cambios_desde_db = true;
    } else if date_db == date_local {
        println!("Pesables sincronizados")
    } else {
        println!("Ultimo actualizado: pesables local");
    }
    Ok(hay_cambios_desde_db)
}
pub async fn update_productos_from_db(
    productos_local: &mut Vec<Producto>,
    path_productos: &str,
    db: &DatabaseConnection,
) -> Res<bool> {
    let mut hay_cambios_desde_db = false;
    let prods_db_model = entity::producto::Entity::find().all(db).await?;
    let date_local = get_updated_time_file(path_productos)?;
    let mut date_db = DateTimeUtc::MIN_UTC;
    if prods_db_model.len() > 0 {
        date_db = get_updated_time_db_producto(prods_db_model.clone()).await;
    }
    if date_local < date_db {
        productos_local.clear();
        println!("Ultimo actualizado: productos de bases de datos");
        for i in 0..prods_db_model.len() {
            let b = get_codigos_db_filtrado(db, prods_db_model[i].id).await?;
            productos_local.push(map_model_prod(&prods_db_model[i], b)?);
        }
        hay_cambios_desde_db = true;
    } else if date_db == date_local {
        println!("Productos sincronizados")
    } else {
        println!("Ultimo actualizado: productos local");
    }
    Ok(hay_cambios_desde_db)
}

fn map_model_prod(prod: &entity::producto::Model, cods: Vec<i64>) -> Res<Producto> {
    let mut parts = prod.presentacion.split(' ');
    let p1 = parts.next().unwrap();
    let p2 = parts.next().unwrap();
    let presentacion = match p2 {
        "Gr" => Presentacion::Gr(p1.parse()?),
        "Un" => Presentacion::Un(p1.parse()?),
        "Lt" => Presentacion::Lt(p1.parse()?),
        "Ml" => Presentacion::Ml(p1.parse()?),
        "CC" => Presentacion::CC(p1.parse()?),
        "Kg" => Presentacion::Kg(p1.parse()?),
        _ => return Err(SizeSelecionError.into()),
    };
    Ok(Producto::new(
        prod.id,
        cods,
        prod.precio_de_venta,
        prod.porcentaje,
        prod.precio_de_costo,
        prod.tipo_producto.clone(),
        prod.marca.clone(),
        prod.variedad.clone(),
        presentacion,
    ))
}
fn map_model_rub(rub: &entity::rubro::Model) -> Rubro {
    Rubro {
        id: rub.id,
        monto: rub.monto,
        descripcion: rub.descripcion.clone(),
    }
}

fn map_model_pes(pes: &entity::pesable::Model) -> Pesable {
    Pesable {
        id: pes.id,
        codigo: pes.codigo,
        precio_peso: pes.precio_peso,
        porcentaje: pes.porcentaje,
        costo_kilo: pes.costo_kilo,
        descripcion: pes.descripcion.clone(),
    }
}
pub async fn cargar_todos_los_productos(
    productos: &Vec<Valuable>,
    db: &DatabaseConnection,
) -> Result<(), DbErr> {
    for producto in productos {
        match producto {
            Valuable::Prod(a) => {
                let encontrado = entity::producto::Entity::find_by_id(a.1.get_id())
                    .one(db)
                    .await?;
                let mut model: ActiveModel;
                let codigo_prod: i64;
                match encontrado {
                    Some(m) => {
                        model = m.into();
                        model.marca = Set(a.1.marca.clone());
                        model.porcentaje = Set(a.1.porcentaje);
                        model.precio_de_costo = Set(a.1.precio_de_costo);
                        model.precio_de_venta = Set(a.1.precio_de_venta);
                        model.presentacion = Set(a.1.presentacion.to_string());
                        model.tipo_producto = Set(a.1.tipo_producto.clone());
                        model.updated_at = Set(Utc::now().naive_utc());
                        model.variedad = Set(a.1.variedad.clone());
                        codigo_prod = model.id.clone().unwrap();
                        model.update(db).await?;
                    }
                    None => {
                        model = producto::ActiveModel {
                            precio_de_venta: Set(a.1.precio_de_venta),
                            porcentaje: Set(a.1.porcentaje),
                            precio_de_costo: Set(a.1.precio_de_costo),
                            tipo_producto: Set(a.1.tipo_producto.clone()),
                            marca: Set(a.1.marca.clone()),
                            variedad: Set(a.1.variedad.clone()),
                            presentacion: Set(a.1.presentacion.to_string()),
                            updated_at: Set(Utc::now().naive_utc()),
                            ..Default::default()
                        };

                        codigo_prod = entity::producto::Entity::insert(model)
                            .exec(db)
                            .await?
                            .last_insert_id;
                    }
                }
                let db_codes = entity::codigo_barras::Entity::find()
                    .filter(
                        Condition::all()
                            .add(entity::codigo_barras::Column::Producto.eq(codigo_prod)),
                    )
                    .all(db)
                    .await?;
                if db_codes.len() == a.1.codigos_de_barras.len() {
                    for i in 0..db_codes.len() {
                        entity::codigo_barras::ActiveModel {
                            id: Set(db_codes[i].id),
                            codigo: Set(a.1.codigos_de_barras[i]),
                            producto: Set(db_codes[i].producto),
                        }
                        .update(db)
                        .await?;
                    }
                } else {
                    entity::codigo_barras::Entity::delete_many()
                        .filter(
                            Condition::all()
                                .add(entity::codigo_barras::Column::Producto.eq(codigo_prod)),
                        )
                        .exec(db)
                        .await?;
                    let codigos_model: Vec<codigo_barras::ActiveModel> =
                        a.1.codigos_de_barras
                            .iter()
                            .map(|x| codigo_barras::ActiveModel {
                                codigo: Set(*x),
                                producto: Set(codigo_prod),
                                ..Default::default()
                            })
                            .collect();

                    if codigos_model.len() > 1 {
                        entity::codigo_barras::Entity::insert_many(codigos_model)
                            .exec(db)
                            .await?;
                    } else if codigos_model.len() == 1 {
                        entity::codigo_barras::Entity::insert(codigos_model[0].to_owned())
                            .exec(db)
                            .await?;
                    }
                }
            }
            _ => (),
        }
    }
    Ok(())
}
pub async fn cargar_todos_los_pesables(
    productos: &Vec<Valuable>,
    db: &DatabaseConnection,
) -> Result<(), DbErr> {
    for i in productos {
        match i {
            Valuable::Pes(a) => {
                let model = pesable::ActiveModel {
                    codigo: Set(a.1.codigo),
                    precio_peso: Set(a.1.precio_peso),
                    porcentaje: Set(a.1.porcentaje),
                    costo_kilo: Set(a.1.costo_kilo),
                    descripcion: Set(a.1.descripcion.clone()),
                    updated_at: Set(Utc::now().naive_utc()),
                    id: Set(a.1.id),
                };
                if entity::pesable::Entity::find_by_id(a.1.id)
                    .one(db)
                    .await?
                    .is_some()
                {
                    model.update(db).await?;
                } else {
                    model.insert(db).await?;
                }
            }
            _ => (),
        }
    }
    Ok(())
}
pub async fn cargar_todos_los_rubros(
    productos: &Vec<Valuable>,
    db: &DatabaseConnection,
) -> Result<(), DbErr> {
    for i in productos {
        match i {
            Valuable::Rub(a) => {
                let model = entity::rubro::ActiveModel {
                    id: Set(a.1.id),
                    monto: Set(a.1.monto),
                    descripcion: Set(a.1.descripcion.clone()),
                    updated_at: Set(Utc::now().naive_utc()),
                };
                if entity::rubro::Entity::find_by_id(a.1.id)
                    .one(db)
                    .await?
                    .is_some()
                {
                    model.update(db).await?;
                } else {
                    model.insert(db).await?;
                }
            }
            _ => (),
        }
    }
    Ok(())
}
pub async fn cargar_todos_los_valuables(productos: Vec<Valuable>) -> Result<(), DbErr> {
    println!("Guardando productos en DB");
    let db = Database::connect("postgres://postgres:L33tsupa@localhost:5432/Tauri").await?;
    cargar_todos_los_productos(&productos, &db).await?;
    cargar_todos_los_pesables(&productos, &db).await?;
    cargar_todos_los_rubros(&productos, &db).await?;
    Ok(())
}
pub async fn cargar_todos_los_provs(proveedores: Vec<Proveedor>) -> Result<(), DbErr> {
    let db = Database::connect("postgres://postgres:L33tsupa@localhost:5432/Tauri").await?;
    let provs: Vec<proveedor::ActiveModel> = proveedores
        .iter()
        .map(|x| {
            let contacto = match x.get_contacto() {
                Some(a) => Some(*a),
                None => None,
            };
            proveedor::ActiveModel {
                id: Set(*x.get_id()),
                nombre: Set(x.get_nombre().clone()),
                contacto: Set(contacto),
            }
        })
        .collect();
    proveedor::Entity::insert_many(provs).exec(&db).await?;

    Ok(())
}

pub async fn cargar_todas_las_relaciones_prod_prov(relaciones:Vec<RelacionProdProv>) -> Result<(),DbErr> {
        let db = Database::connect("postgres://postgres:L33tsupa@localhost:5432/Tauri").await?;
        let mut relaciones_model = Vec::new();
        for x in relaciones {
            let a = entity::producto::Entity::find_by_id(x.get_id_producto())
                .one(&db)
                .await?
                .is_some();
            let b = entity::proveedor::Entity::find_by_id(x.get_id_proveedor())
                .one(&db)
                .await?
                .is_some();
            if a && b {
                relaciones_model.push(entity::relacion_prod_prov::ActiveModel {
                    producto: Set(x.get_id_producto()),
                    proveedor: Set(x.get_id_proveedor()),
                    codigo: Set(x.get_codigo_interno()),
                    ..Default::default()
                })
            }
        }
        entity::relacion_prod_prov::Entity::insert_many(relaciones_model)
            .exec(&db)
            .await?;
        Ok(())
    }
