use std::{marker::PhantomData, sync::Arc};

use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldError, FieldResult, GraphQLObject, Value,
};

use super::CustomError;
use crate::{db::MeigenDatabase, model, Synced};

#[derive(GraphQLObject)]
#[graphql(description = "A great sentence someone created via Discord Bot")]
pub struct Meigen {
    pub id: i32,
    pub author: String,
    pub content: String,
}

impl From<model::Meigen> for Meigen {
    fn from(m: model::Meigen) -> Self {
        Self {
            id: m.id as i32,
            author: m.author,
            content: m.content,
        }
    }
}

impl From<Meigen> for model::Meigen {
    fn from(m: Meigen) -> Self {
        Self {
            id: m.id as u32,
            author: m.author,
            content: m.content,
        }
    }
}

pub(crate) fn schema<D>() -> Schema<D>
where
    D: MeigenDatabase,
{
    Schema::new(Query::new(), EmptyMutation::new(), EmptySubscription::new())
}

pub(crate) struct Context<D> {
    pub(crate) db: Synced<D>,
}

impl<D> Clone for Context<D>
where
    D: MeigenDatabase,
{
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
        }
    }
}

impl<D> juniper::Context for Context<D> where D: MeigenDatabase {}

pub(crate) struct Query<D> {
    _phantom_db: PhantomData<fn() -> D>,
}

impl<D> Query<D> {
    fn new() -> Self {
        Self {
            _phantom_db: PhantomData,
        }
    }
}

fn into_field_error(e: CustomError) -> FieldError {
    FieldError::new(e.describe(), Value::Null)
}

#[graphql_object(context = Context<D>)]
impl<D> Query<D>
where
    D: MeigenDatabase,
{
    async fn get(context: &Context<D>, id: i32) -> FieldResult<Option<Meigen>> {
        match super::get(id as u32, Arc::clone(&context.db)).await {
            Ok(Some(v)) => Ok(Some(v.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(into_field_error(e)),
        }
    }
}

type Schema<D> =
    juniper::RootNode<'static, Query<D>, EmptyMutation<Context<D>>, EmptySubscription<Context<D>>>;
