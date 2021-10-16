use std::{convert::TryInto, marker::PhantomData, sync::Arc};

use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldError, FieldResult, GraphQLInputObject,
    GraphQLObject, Value,
};

use super::CustomError;
use crate::{db::MeigenDatabase, model, Synced};

#[derive(GraphQLObject)]
#[graphql(description = "A great sentence someone created via Discord Bot")]
struct Meigen {
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

#[derive(GraphQLInputObject)]
struct RandomRequest {
    count: Option<i32>,
}

#[derive(GraphQLInputObject)]
struct SearchRequest {
    offset: Option<i32>,
    limit: Option<i32>,
    author: Option<String>,
    content: Option<String>,
}

type Schema<D> =
    juniper::RootNode<'static, Query<D>, EmptyMutation<Context<D>>, EmptySubscription<Context<D>>>;

pub(crate) fn schema<D: MeigenDatabase>() -> Schema<D> {
    Schema::new(Query::new(), EmptyMutation::new(), EmptySubscription::new())
}

pub(crate) struct Context<D> {
    pub(crate) db: Synced<D>,
}

// #[derive(Clone)] requires D: Clone which is not actually needed.
impl<D> Clone for Context<D> {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
        }
    }
}

impl<D> juniper::Context for Context<D> {}

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

macro_rules! convert_opt_int {
    ($value:expr, $field_name:literal) => {
        match $value {
            Some(t) => Some(t.try_into().map_err(|_| {
                FieldError::new(concat!($field_name, "is negative or too big"), Value::Null)
            })?),

            None => None,
        }
    };
}

#[graphql_object(context = Context<D>)]
impl<D: MeigenDatabase> Query<D> {
    async fn get(context: &Context<D>, id: i32) -> FieldResult<Option<Meigen>> {
        match super::get(id as u32, Arc::clone(&context.db)).await {
            Ok(Some(v)) => Ok(Some(v.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(into_field_error(e)),
        }
    }

    async fn random(context: &Context<D>, count: Option<i32>) -> FieldResult<Vec<Meigen>> {
        let count = convert_opt_int!(count, "count");

        match super::random(super::RandomRequest { count }, Arc::clone(&context.db)).await {
            Ok(v) => Ok(v.into_iter().map(From::from).collect()),
            Err(e) => Err(into_field_error(e)),
        }
    }

    async fn search(context: &Context<D>, option: SearchRequest) -> FieldResult<Vec<Meigen>> {
        let option = super::SearchRequest {
            offset: convert_opt_int!(option.offset, "offset"),
            limit: convert_opt_int!(option.limit, "limit"),
            author: option.author,
            content: option.content,
        };

        match super::search(option, Arc::clone(&context.db)).await {
            Ok(v) => Ok(v.into_iter().map(From::from).collect()),
            Err(e) => Err(into_field_error(e)),
        }
    }
}
