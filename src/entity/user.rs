use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait,
    DatabaseConnection, DbErr, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, EntityTrait,
    EnumIter, ModelTrait, PrimaryKeyTrait, QueryFilter, Schema,
};
use tracing::{debug, warn};

use crate::entity::user;

#[derive(Debug, Clone, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub surname: String,
}
#[derive(Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub async fn create_tables(db: &DatabaseConnection) -> Result<(), DbErr> {
    let schema = Schema::new(db.get_database_backend());
    let stmt = schema.create_table_from_entity(user::Entity);

    db.execute(db.get_database_backend().build(&stmt)).await?;

    Ok(())
}

pub async fn insert_user(db: &DatabaseConnection, user: ActiveModel) -> Result<(), DbErr> {
    user.insert(db).await?;

    Ok(())
}

pub async fn insert_default_users(db: &DatabaseConnection) -> Result<(), DbErr> {
    let users = vec![
        user::ActiveModel {
            name: Set("Emil".to_string()),
            surname: Set("Hans".to_string()),
            ..Default::default()
        },
        user::ActiveModel {
            name: Set("Mustermann".to_string()),
            surname: Set("Max".to_string()),
            ..Default::default()
        },
        user::ActiveModel {
            name: Set("Tisch".to_string()),
            surname: Set("Roman".to_string()),
            ..Default::default()
        },
    ];

    for user in users {
        user.insert(db).await?;
    }

    Ok(())
}

pub async fn get_all_users(db: &DatabaseConnection) -> Result<Vec<user::Model>, DbErr> {
    user::Entity::find().all(db).await
}

pub async fn find_user_by_name_surname(
    db: &DatabaseConnection,
    name: &str,
    surname: &str,
) -> Result<Vec<user::Model>, DbErr> {
    user::Entity::find()
        .filter(user::Column::Name.eq(name))
        .filter(user::Column::Surname.eq(surname))
        .all(db)
        .await
}

pub async fn update_user(
    db: &DatabaseConnection,
    id: i32,
    name: &str,
    surname: &str,
) -> Result<(), DbErr> {
    let user_to_update = user::ActiveModel {
        id: Set(id),
        name: Set(name.to_owned()),
        surname: Set(surname.to_owned()),
    };

    match user_to_update.update(db).await {
        Ok(_) => debug!("Updated user (ID: {}) successfully", id),
        Err(DbErr::RecordNotFound(_)) => warn!("User with ID {} not found", id),
        Err(e) => return Err(e),
    }

    Ok(())
}

pub async fn delete_user(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
    let user_to_delete = user::Entity::find_by_id(id).one(db).await?;

    if let Some(user) = user_to_delete {
        user.delete(db).await?;
    } else {
        warn!("User with ID {} not found", id)
    }

    Ok(())
}
