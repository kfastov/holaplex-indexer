//! Query utilities for collections.

use anyhow::Context;
use chrono::{DateTime, Utc};
use diesel::{
    expression::{operators::Eq, AsExpression, NonAggregate},
    pg::Pg,
    prelude::*,
    query_builder::{QueryFragment, QueryId},
    query_source::joins::{Inner, Join, JoinOn},
    serialize::ToSql,
    sql_types::{Array, Integer, Nullable, Text, Timestamp},
};
use sea_query::{Expr, Iden, Order, PostgresQueryBuilder, Query};

use crate::{
    db::{
        custom_types::{CollectionSort, OrderDirection},
        models::{DolphinStats as DolphinStatsDB, Nft, NftActivity},
        queries::metadatas::NFT_COLUMNS,
        tables::{current_metadata_owners, metadata_collection_keys, metadata_jsons, metadatas},
        Connection,
    },
    error::Result,
    prelude::*,
};

#[derive(Iden)]
#[allow(missing_docs)]
enum DolphinStats {
    Table,
    CollectionSymbol,
    #[iden(rename = "floor_1d")]
    Floor1d,
    #[iden(rename = "floor_7d")]
    Floor7d,
    #[iden(rename = "floor_30d")]
    Floor30d,
    #[iden(rename = "listed_1d")]
    Listed1d,
    #[iden(rename = "listed_7d")]
    Listed7d,
    #[iden(rename = "listed_30d")]
    Listed30d,
    #[iden(rename = "volume_1d")]
    Volume1d,
    #[iden(rename = "volume_7d")]
    Volume7d,
    #[iden(rename = "volume_30d")]
    Volume30d,
    #[iden(rename = "last_floor_1d")]
    LastFloor1d,
    #[iden(rename = "last_floor_7d")]
    LastFloor7d,
    #[iden(rename = "last_floor_30d")]
    LastFloor30d,
    #[iden(rename = "last_listed_1d")]
    LastListed1d,
    #[iden(rename = "last_listed_7d")]
    LastListed7d,
    #[iden(rename = "last_listed_30d")]
    LastListed30d,
    #[iden(rename = "last_volume_1d")]
    LastVolume1d,
    #[iden(rename = "last_volume_7d")]
    LastVolume7d,
    #[iden(rename = "last_volume_30d")]
    LastVolume30d,
    #[iden(rename = "change_floor_1d")]
    ChangeFloor1d,
    #[iden(rename = "change_floor_7d")]
    ChangeFloor7d,
    #[iden(rename = "change_floor_30d")]
    ChangeFloor30d,
    #[iden(rename = "change_volume_1d")]
    ChangeVolume1d,
    #[iden(rename = "change_volume_7d")]
    ChangeVolume7d,
    #[iden(rename = "change_volume_30d")]
    ChangeVolume30d,
    #[iden(rename = "change_listed_1d")]
    ChangeListed1d,
    #[iden(rename = "change_listed_7d")]
    ChangeListed7d,
    #[iden(rename = "change_listed_30d")]
    ChangeListed30d,
}

#[derive(Iden)]
#[allow(missing_docs)]
enum Collections {
    Table,
    Id,
}

/// Query collection by address
///
/// # Errors
/// returns an error when the underlying queries throw an error
pub fn get<A: AsExpression<Text>>(conn: &Connection, address: A) -> Result<Option<Nft>>
where
    <A as AsExpression<Text>>::Expression: QueryId
        + QueryFragment<Pg>
        + AppearsOnTable<
            JoinOn<
                Join<
                    JoinOn<
                        Join<
                            JoinOn<
                                Join<metadatas::table, metadata_jsons::table, Inner>,
                                Eq<
                                    metadatas::columns::address,
                                    metadata_jsons::columns::metadata_address,
                                >,
                            >,
                            metadata_collection_keys::table,
                            Inner,
                        >,
                        Eq<
                            metadata_collection_keys::columns::collection_address,
                            metadatas::columns::mint_address,
                        >,
                    >,
                    current_metadata_owners::table,
                    Inner,
                >,
                Eq<
                    current_metadata_owners::columns::mint_address,
                    metadatas::columns::mint_address,
                >,
            >,
        > + NonAggregate,
{
    metadatas::table
        .inner_join(
            metadata_jsons::table.on(metadatas::address.eq(metadata_jsons::metadata_address)),
        )
        .inner_join(
            metadata_collection_keys::table
                .on(metadata_collection_keys::collection_address.eq(metadatas::mint_address)),
        )
        .inner_join(
            current_metadata_owners::table
                .on(current_metadata_owners::mint_address.eq(metadatas::mint_address)),
        )
        .filter(metadata_collection_keys::collection_address.eq(address))
        .filter(metadata_collection_keys::verified.eq(true))
        .select(NFT_COLUMNS)
        .first::<Nft>(conn)
        .optional()
        .context("Failed to load Collection NFT by collection address")
}

/// Query collections ordered by volume
///
/// # Errors
/// returns an error when the underlying queries throw an error
pub fn by_volume(
    conn: &Connection,
    addresses: impl ToSql<Nullable<Array<Text>>, Pg>,
    order_direction: OrderDirection,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    limit: impl ToSql<Integer, Pg>,
    offset: impl ToSql<Integer, Pg>,
) -> Result<Vec<Nft>> {
    diesel::sql_query(make_by_volume_query_string(order_direction))
        .bind(addresses)
        .bind::<Timestamp, _>(start_date.naive_utc())
        .bind::<Timestamp, _>(end_date.naive_utc())
        .bind(limit)
        .bind(offset)
        .load(conn)
        .context("Failed to load collections by volume")
}

fn make_by_volume_query_string(order_direction: OrderDirection) -> String {
    format!(
        r"
        WITH collection_volumes AS (
            (SELECT SUM(purchases.price)::numeric as total_volume,
            metadata_collection_keys.collection_address as collection_address,
            null as collection_id
            FROM purchases
            INNER JOIN metadata_collection_keys ON (metadata_collection_keys.metadata_address = purchases.metadata)
            WHERE
            ($1 IS NULL OR metadata_collection_keys.collection_address = ANY($1))
            AND purchases.created_at >= $2
            AND purchases.created_at <= $3
            AND purchases.marketplace_program = 'M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K'
            GROUP BY collection_address
            LIMIT $4)
            UNION ALL
            (SELECT SUM(purchases.price)::numeric as total_volume,
            null as collection_address,
            me_metadata_collections.collection_id::text as collection_id
            FROM purchases
            INNER JOIN me_metadata_collections ON (me_metadata_collections.metadata_address = purchases.metadata)
            WHERE
            ($1 IS NULL OR me_metadata_collections.collection_id::text = ANY($1))
            AND purchases.created_at >= $2
            AND purchases.created_at <= $3
            AND purchases.marketplace_program = 'M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K'
            GROUP BY collection_id
            LIMIT $4)
            ORDER BY total_volume {order_direction}
            LIMIT $4
            OFFSET $5
        )         SELECT
                    address,
                    name,
                    seller_fee_basis_points,
                    update_authority_address,
                    mint_address,
                    primary_sale_happened,
                    uri,
                    slot,
                    description,
                    image,
                    animation_url,
                    external_url,
                    category,
                    model,
                    token_account_address
                    from
                        (SELECT
                            metadatas.address,
                            metadatas.name,
                            metadatas.seller_fee_basis_points,
                            metadatas.update_authority_address,
                            metadatas.mint_address,
                            metadatas.primary_sale_happened,
                            metadatas.uri,
                            metadatas.slot,
                            metadata_jsons.description,
                            metadata_jsons.image,
                            metadata_jsons.animation_url,
                            metadata_jsons.external_url,
                            metadata_jsons.category,
                            metadata_jsons.model,
                            current_metadata_owners.token_account_address,
                            collection_volumes.total_volume
                        FROM metadatas
                        INNER JOIN metadata_jsons ON (metadata_jsons.metadata_address = metadatas.address)
                        INNER JOIN collection_volumes ON (collection_volumes.collection_address = metadatas.mint_address)
                        INNER JOIN current_metadata_owners ON (current_metadata_owners.mint_address = metadatas.mint_address)
                        UNION ALL
                        SELECT
                            me_collections.id::text as address,
                            me_collections.name as name,
                            0 as seller_fee_basis_points,
                            '' as update_authority_address,
                            me_collections.id::text as mint_address,
                            false as primary_sale_happened,
                            '' as uri,
                            0 as slot,
                            '' as description,
                            me_collections.image as image,
                            '' as animation_url,
                            '' as external_url,
                            '' as category,
                            '' as model,
                            '' as token_account_address,
                            collection_volumes.total_volume
                        FROM collection_volumes
                        INNER JOIN me_collections  ON (collection_volumes.collection_id = me_collections.id::text)
                        ) as A
                    ORDER BY total_volume {order_direction};
    -- $1: addresses::text[]
    -- $2: start date::timestamp
    -- $3: end date::timestamp
    -- $4: limit::integer
    -- $5: offset::integer",
        order_direction = order_direction
    )
}

/// Query collections ordered by market cap
///
/// # Errors
/// returns an error when the underlying queries throw an error
pub fn by_market_cap(
    conn: &Connection,
    addresses: impl ToSql<Nullable<Array<Text>>, Pg>,
    order_direction: OrderDirection,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    limit: impl ToSql<Integer, Pg>,
    offset: impl ToSql<Integer, Pg>,
) -> Result<Vec<Nft>> {
    diesel::sql_query(make_by_market_cap_query_string(order_direction))
        .bind(addresses)
        .bind::<Timestamp, _>(start_date.naive_utc())
        .bind::<Timestamp, _>(end_date.naive_utc())
        .bind(limit)
        .bind(offset)
        .load(conn)
        .context("Failed to load collections by market cap")
}

#[allow(clippy::too_many_lines)]
fn make_by_market_cap_query_string(order_direction: OrderDirection) -> String {
    format!(
        r"
        WITH market_caps AS (
            (SELECT MIN(listings.price)::numeric * collection_stats.nft_count::numeric as market_cap,
            collection_stats.collection_address as collection_address, null as collection_id
            FROM listings
            INNER JOIN metadata_collection_keys ON (metadata_collection_keys.metadata_address = listings.metadata)
            INNER JOIN collection_stats ON (collection_stats.collection_address = metadata_collection_keys.collection_address)
            WHERE listings.purchase_id IS NULL
            AND ($1 IS NULL OR metadata_collection_keys.collection_address = ANY($1))
            AND listings.canceled_at IS NULL
            AND listings.created_at >= $2
            AND listings.created_at <= $3
            AND listings.marketplace_program = 'M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K'
            GROUP BY collection_stats.collection_address
            LIMIT $4)
            UNION ALL
            (SELECT MIN(listings.price)::numeric * me_collection_stats.nft_count::numeric as market_cap,
            null as collection_address, me_collection_stats.collection_id as collection_id
            FROM listings
            INNER JOIN me_metadata_collections ON (me_metadata_collections.metadata_address = listings.metadata)
            INNER JOIN me_collection_stats ON (me_collection_stats.collection_id = me_metadata_collections.collection_id)
            WHERE listings.purchase_id IS NULL
            AND ($1 IS NULL OR me_metadata_collections.collection_id::text = ANY($1))
            AND listings.canceled_at IS NULL
            AND listings.created_at >= $2
            AND listings.created_at <= $3
            AND listings.marketplace_program = 'M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K'
            GROUP BY me_collection_stats.collection_id
            LIMIT $4)
            ORDER BY market_cap {order_direction}
            LIMIT $4
            OFFSET $5
        )   SELECT
                address,
                name,
                seller_fee_basis_points,
                mint_address,
                primary_sale_happened,
                update_authority_address,
                uri,
                slot,
                description,
                image,
                animation_url,
                external_url,
                category,
                model,
                token_account_address
                from
                    (
                        SELECT
                            metadatas.address,
                            metadatas.name,
                            metadatas.seller_fee_basis_points,
                            metadatas.update_authority_address,
                            metadatas.mint_address,
                            metadatas.primary_sale_happened,
                            metadatas.uri,
                            metadatas.slot,
                            metadata_jsons.description,
                            metadata_jsons.image,
                            metadata_jsons.animation_url,
                            metadata_jsons.external_url,
                            metadata_jsons.category,
                            metadata_jsons.model,
                            current_metadata_owners.token_account_address,
                            market_caps.market_cap::numeric
                            FROM metadatas
                            INNER JOIN metadata_jsons ON (metadata_jsons.metadata_address = metadatas.address)
                            INNER JOIN market_caps ON (market_caps.collection_address = metadatas.mint_address)
                            INNER JOIN current_metadata_owners ON (current_metadata_owners.mint_address = metadatas.mint_address)
                        UNION ALL
                        SELECT
                            me_collections.id::text as address,
                            COALESCE(me_collections.name, '') as name,
                            0 as seller_fee_basis_points,
                            '' as update_authority_address,
                            me_collections.id::text as mint_address,
                            false as primary_sale_happened,
                            '' as uri,
                            0 as slot,
                            '' as description,
                            me_collections.image as image,
                            '' as animation_url,
                            '' as external_url,
                            '' as category,
                            '' as model,
                            '' as token_account_address,
                            market_caps.market_cap::numeric
                        FROM me_collections
				        INNER JOIN market_caps ON (market_caps.collection_id = me_collections.id)
                    ) as M
                    ORDER BY market_cap {order_direction};
    -- $1: addresses::text[]
    -- $2: start date::timestamp
    -- $3: end date::timestamp
    -- $4: limit::integer
    -- $5: offset::integer",
        order_direction = order_direction
    )
}

const COLLECTION_ACTIVITES_QUERY: &str = r"
SELECT listings.id as id, metadata, auction_house, price, created_at, marketplace_program,
    array[seller] as wallets,
    array[twitter_handle_name_services.twitter_handle] as wallet_twitter_handles,
    'listing' as activity_type
        FROM listings
        LEFT JOIN twitter_handle_name_services ON(twitter_handle_name_services.wallet_address = listings.seller)
        INNER JOIN metadata_collection_keys ON(metadata_collection_keys.metadata_address = listings.metadata)
        WHERE metadata_collection_keys.collection_address = $1
        AND listings.auction_house != '3o9d13qUvEuuauhFrVom1vuCzgNsJifeaBYDPquaT73Y'
        AND ('LISTINGS' = ANY($2) OR $2 IS NULL)
	UNION
	SELECT listings.id as id, metadata, auction_house, price, created_at, marketplace_program,
    array[seller] as wallets,
    array[twitter_handle_name_services.twitter_handle] as wallet_twitter_handles,
    'listing' as activity_type
        FROM listings
        LEFT JOIN twitter_handle_name_services ON(twitter_handle_name_services.wallet_address = listings.seller)
        INNER JOIN me_metadata_collections ON(me_metadata_collections.metadata_address = listings.metadata)
        WHERE me_metadata_collections.collection_id::text = $1
        AND ('LISTINGS' = ANY($2) OR $2 IS NULL)
    UNION
    SELECT purchases.id as id, metadata, auction_house, price, created_at, marketplace_program,
    array[seller, buyer] as wallets,
    array[sth.twitter_handle, bth.twitter_handle] as wallet_twitter_handles,
    'purchase' as activity_type
        FROM purchases
        LEFT JOIN twitter_handle_name_services sth ON(sth.wallet_address = purchases.seller)
        LEFT JOIN twitter_handle_name_services bth ON(bth.wallet_address = purchases.buyer)
        INNER JOIN metadata_collection_keys ON(metadata_collection_keys.metadata_address = purchases.metadata)
        WHERE metadata_collection_keys.collection_address = $1
        AND ('PURCHASES' = ANY($2) OR $2 IS NULL)
	UNION
    SELECT purchases.id as id, metadata, auction_house, price, created_at, marketplace_program,
    array[seller, buyer] as wallets,
    array[sth.twitter_handle, bth.twitter_handle] as wallet_twitter_handles,
    'purchase' as activity_type
        FROM purchases
        LEFT JOIN twitter_handle_name_services sth ON(sth.wallet_address = purchases.seller)
        LEFT JOIN twitter_handle_name_services bth ON(bth.wallet_address = purchases.buyer)
        INNER JOIN me_metadata_collections ON(me_metadata_collections.metadata_address = purchases.metadata)
        WHERE me_metadata_collections.collection_id::text = $1
        AND ('PURCHASES' = ANY($2) OR $2 IS NULL)
    UNION
    SELECT offers.id as id, metadata, auction_house, price, created_at, marketplace_program,
    array[buyer] as wallets,
    array[bth.twitter_handle] as wallet_twitter_handles,
    'offer' as activity_type
        FROM offers
        LEFT JOIN twitter_handle_name_services bth ON(bth.wallet_address = offers.buyer)
        INNER JOIN metadata_collection_keys ON(metadata_collection_keys.metadata_address = offers.metadata)
        WHERE metadata_collection_keys.collection_address = $1
        AND offers.purchase_id IS NULL
        AND offers.auction_house != '3o9d13qUvEuuauhFrVom1vuCzgNsJifeaBYDPquaT73Y'
        AND ('OFFERS' = ANY($2) OR $2 IS NULL)
	UNION
    SELECT offers.id as id, metadata, auction_house, price, created_at, marketplace_program,
    array[buyer] as wallets,
    array[bth.twitter_handle] as wallet_twitter_handles,
    'offer' as activity_type
        FROM offers
        LEFT JOIN twitter_handle_name_services bth ON(bth.wallet_address = offers.buyer)
        INNER JOIN me_metadata_collections ON(me_metadata_collections.metadata_address = offers.metadata)
        WHERE me_metadata_collections.collection_id::text = $1
        AND offers.purchase_id IS NULL
        AND ('OFFERS' = ANY($2) OR $2 IS NULL)
    ORDER BY created_at DESC
    LIMIT $3
    OFFSET $4;

 -- $1: address::text
 -- $2: event_types::text[]
 -- $3: limit::integer
 -- $4: offset::integer";

/// Load listing, sales, offers activity for a collection
///
/// # Errors
/// This function fails if the underlying SQL query returns an error
pub fn collection_activities(
    conn: &Connection,
    address: impl ToSql<Text, Pg>,
    event_types: impl ToSql<Nullable<Array<Text>>, Pg>,
    limit: impl ToSql<Integer, Pg>,
    offset: impl ToSql<Integer, Pg>,
) -> Result<Vec<NftActivity>> {
    diesel::sql_query(COLLECTION_ACTIVITES_QUERY)
        .bind(address)
        .bind(event_types)
        .bind(limit)
        .bind(offset)
        .load(conn)
        .context("Failed to load collection activities")
}

/// Input parameters for the `trending` query.
#[derive(Debug)]
pub struct TrendingQueryOptions {
    /// Sort by Price or Listed at
    pub sort_by: CollectionSort,
    /// Order the resulting rows by 'Asc' or 'Desc'
    pub order: Option<Order>,
    /// Limit the number of returned rows
    pub limit: u64,
    /// Skip the first `n` resulting rows
    pub offset: u64,
}

impl From<CollectionSort> for DolphinStats {
    fn from(sort: CollectionSort) -> Self {
        match sort {
            CollectionSort::OneDayFloorPrice => DolphinStats::Floor1d,
            CollectionSort::SevenDayFloorPrice => DolphinStats::Floor7d,
            CollectionSort::ThirtyDayFloorPrice => DolphinStats::Floor30d,
            CollectionSort::OneDayVolume => DolphinStats::Volume1d,
            CollectionSort::SevenDayVolume => DolphinStats::Volume7d,
            CollectionSort::ThirtyDayVolume => DolphinStats::Volume30d,
            CollectionSort::OneDayListedCount => DolphinStats::Listed1d,
            CollectionSort::SevenDayListedCount => DolphinStats::Listed7d,
            CollectionSort::ThirtyDayListedCount => DolphinStats::Listed30d,
        }
    }
}

/// Handles queries for trending collections
///
/// # Errors
/// returns an error when the underlying queries throw an error
pub fn trends(conn: &Connection, options: TrendingQueryOptions) -> Result<Vec<DolphinStatsDB>> {
    let TrendingQueryOptions {
        sort_by,
        order,
        limit,
        offset,
    } = options;

    let sort_by: DolphinStats = sort_by.into();

    let order = order.unwrap_or(Order::Desc);

    let query = Query::select()
        .columns(vec![
            (DolphinStats::Table, DolphinStats::CollectionSymbol),
            (DolphinStats::Table, DolphinStats::Floor1d),
            (DolphinStats::Table, DolphinStats::Floor7d),
            (DolphinStats::Table, DolphinStats::Floor30d),
            (DolphinStats::Table, DolphinStats::Listed1d),
            (DolphinStats::Table, DolphinStats::Listed7d),
            (DolphinStats::Table, DolphinStats::Listed30d),
            (DolphinStats::Table, DolphinStats::Volume1d),
            (DolphinStats::Table, DolphinStats::Volume7d),
            (DolphinStats::Table, DolphinStats::Volume30d),
            (DolphinStats::Table, DolphinStats::LastFloor1d),
            (DolphinStats::Table, DolphinStats::LastFloor7d),
            (DolphinStats::Table, DolphinStats::LastFloor30d),
            (DolphinStats::Table, DolphinStats::LastListed1d),
            (DolphinStats::Table, DolphinStats::LastListed7d),
            (DolphinStats::Table, DolphinStats::LastListed30d),
            (DolphinStats::Table, DolphinStats::LastVolume1d),
            (DolphinStats::Table, DolphinStats::LastVolume7d),
            (DolphinStats::Table, DolphinStats::LastVolume30d),
            (DolphinStats::Table, DolphinStats::ChangeFloor1d),
            (DolphinStats::Table, DolphinStats::ChangeFloor7d),
            (DolphinStats::Table, DolphinStats::ChangeFloor30d),
            (DolphinStats::Table, DolphinStats::ChangeVolume1d),
            (DolphinStats::Table, DolphinStats::ChangeVolume7d),
            (DolphinStats::Table, DolphinStats::ChangeVolume30d),
            (DolphinStats::Table, DolphinStats::ChangeListed1d),
            (DolphinStats::Table, DolphinStats::ChangeListed7d),
            (DolphinStats::Table, DolphinStats::ChangeListed30d),
        ])
        .from(DolphinStats::Table)
        .inner_join(
            Collections::Table,
            Expr::tbl(Collections::Table, Collections::Id)
                .equals(DolphinStats::Table, DolphinStats::CollectionSymbol),
        )
        .limit(limit)
        .offset(offset)
        .order_by((DolphinStats::Table, sort_by), order)
        .take();

    let query = query.to_string(PostgresQueryBuilder);

    diesel::sql_query(query)
        .load(conn)
        .context("Failed to load trending collection(s)")
}

// MoonRank queries

const MR_COLLECTION_ACTIVITES_QUERY: &str = r"
SELECT listings.id as id, metadata, auction_house, price, listings.created_at, marketplace_program,
    array[seller] as wallets,
    array[twitter_handle_name_services.twitter_handle] as wallet_twitter_handles,
    'listing' as activity_type
        FROM listings
        LEFT JOIN twitter_handle_name_services ON(twitter_handle_name_services.wallet_address = listings.seller)
        INNER JOIN metadatas on (metadatas.address = listings.metadata)
        INNER JOIN collection_mints ON(collection_mints.mint = metadatas.mint_address)
        WHERE collection_mints.collection_id = $1
        AND listings.auction_house != '3o9d13qUvEuuauhFrVom1vuCzgNsJifeaBYDPquaT73Y'
        AND ('LISTINGS' = ANY($2) OR $2 IS NULL)
    UNION
    SELECT purchases.id as id, metadata, auction_house, price, purchases.created_at, marketplace_program,
    array[seller, buyer] as wallets,
    array[sth.twitter_handle, bth.twitter_handle] as wallet_twitter_handles,
    'purchase' as activity_type
        FROM purchases
        LEFT JOIN twitter_handle_name_services sth ON(sth.wallet_address = purchases.seller)
        LEFT JOIN twitter_handle_name_services bth ON(bth.wallet_address = purchases.buyer)
        INNER JOIN metadatas on (metadatas.address = purchases.metadata)
        INNER JOIN collection_mints ON(collection_mints.mint = metadatas.mint_address)
        WHERE collection_mints.collection_id = $1
        AND ('PURCHASES' = ANY($2) OR $2 IS NULL)
    UNION
    SELECT offers.id as id, metadata, auction_house, price, offers.created_at, marketplace_program,
    array[buyer] as wallets,
    array[bth.twitter_handle] as wallet_twitter_handles,
    'offer' as activity_type
        FROM offers
        LEFT JOIN twitter_handle_name_services bth ON(bth.wallet_address = offers.buyer)
        INNER JOIN metadatas on (metadatas.address = offers.metadata)
        INNER JOIN collection_mints ON(collection_mints.mint = metadatas.mint_address)
        WHERE collection_mints.collection_id = $1
        AND offers.purchase_id IS NULL
        AND offers.auction_house != '3o9d13qUvEuuauhFrVom1vuCzgNsJifeaBYDPquaT73Y'
        AND ('OFFERS' = ANY($2) OR $2 IS NULL)
    ORDER BY created_at DESC
    LIMIT $3
    OFFSET $4;

 -- $1: id::text
 -- $2: event_types::text[]
 -- $3: limit::integer
 -- $4: offset::integer";

/// Load listing, sales, offers activity for a collection
///
/// # Errors
/// This function fails if the underlying SQL query returns an error
pub fn mr_collection_activities(
    conn: &Connection,
    id: impl ToSql<Text, Pg>,
    event_types: impl ToSql<Nullable<Array<Text>>, Pg>,
    limit: impl ToSql<Integer, Pg>,
    offset: impl ToSql<Integer, Pg>,
) -> Result<Vec<NftActivity>> {
    diesel::sql_query(MR_COLLECTION_ACTIVITES_QUERY)
        .bind(id)
        .bind(event_types)
        .bind(limit)
        .bind(offset)
        .load(conn)
        .context("Failed to load collection activities")
}
