use sqlx::PgPool;

#[derive(sqlx::FromRow, Debug)]
pub struct Location {
    pub name: String,
    pub population: i64,
    pub parent: Option<String>,
}

pub async fn get_all_locations(pool: &PgPool) -> Result<Vec<Location>, sqlx::Error> {
    let locations = sqlx::query_as!(
        Location,
        r#"
SELECT name, population, parent
FROM locations
        "#
    )
    .fetch_all(pool)
    .await?;
    Ok(locations)
}

pub async fn get_location(pool: &PgPool, location: &str) -> Result<Option<Location>, sqlx::Error> {
    let location = sqlx::query_as!(
        Location,
        r#"
SELECT name, population, parent
FROM locations
WHERE name = $1
        "#,
        location
    )
    .fetch_optional(pool)
    .await?;
    Ok(location)
}

pub async fn get_parents(pool: &PgPool, location: &str) -> Result<Vec<String>, sqlx::Error> {
    #[derive(sqlx::FromRow, Default)]
    struct Parents {
        #[allow(dead_code)]
        parents: Option<Vec<String>>,
    }

    let parents = sqlx::query_as!(
        Parents,
        r#"
WITH RECURSIVE locations_cte(name, parent, parents) AS (
  SELECT
    locations.name, 
    locations.parent,
    ARRAY[locations.name::TEXT] as parents
  FROM
    locations
  WHERE
    locations.name = $1
  UNION ALL
  SELECT
    locations.name, 
    locations.parent,
    ARRAY_APPEND(locations_cte.parents, locations.name::TEXT)
  FROM
    locations_cte,
    locations
  WHERE
    locations.name = locations_cte.parent
)
SELECT
  parents
FROM
  locations_cte
WHERE
  parent IS NULL
        "#,
        location
    )
    .fetch_optional(pool)
    .await?;

    let parents = parents.unwrap_or_default().parents.unwrap_or_default();
    Ok(parents)
}
