//! Query utilities for looking up  metadatas
use diesel::{
    pg::Pg,
    prelude::*,
    serialize::ToSql,
    sql_types::{Array, Text},
};
use sea_query::{
    Alias, Condition, DynIden, Expr, Iden, JoinType, Order, PostgresQueryBuilder, Query, SeaRc,
    Value,
};
use uuid::Uuid;

use crate::{
    db::{
        custom_types::NftSort,
        models::{Nft, NftActivity},
        tables::{current_metadata_owners, metadata_jsons, metadatas},
        Connection,
    },
    error::prelude::*,
    prelude::Utc,
};

/// Format for incoming filters on attributes
#[derive(Debug)]
pub struct AttributeFilter {
    /// name of trait
    pub trait_type: String,
    /// array of trait values
    pub values: Vec<String>,
}

#[derive(Iden)]
enum Metadatas {
    Table,
    Address,
    Name,
    MintAddress,
    PrimarySaleHappened,
    SellerFeeBasisPoints,
    UpdateAuthorityAddress,
    Uri,
    Slot,
    BurnedAt,
}

#[derive(Iden)]
enum MeMetadataCollections {
    Table,
    CollectionId,
    MetadataAddress,
}

#[derive(Iden)]
enum CollectionMints {
    Table,
    CollectionId,
    Mint,
}

#[derive(Iden)]
enum MetadataJsons {
    Table,
    MetadataAddress,
    Description,
    Image,
    AnimationUrl,
    ExternalUrl,
    Category,
    Model,
}

#[derive(Iden)]
enum CurrentMetadataOwners {
    Table,
    OwnerAddress,
    MintAddress,
    TokenAccountAddress,
}

#[derive(Iden)]
enum Listings {
    Table,
    Price,
    Metadata,
    AuctionHouse,
    MarketplaceProgram,
    Seller,
    PurchaseId,
    CanceledAt,
    Expiry,
    CreatedAt,
}

#[derive(Iden)]
enum MetadataCreators {
    Table,
    CreatorAddress,
    MetadataAddress,
    Verified,
}

#[derive(Iden)]
enum Offers {
    Table,
    Buyer,
    Price,
    Metadata,
    CanceledAt,
    PurchaseId,
    AuctionHouse,
    Expiry,
}

#[derive(Iden)]
enum Attributes {
    Table,
    MetadataAddress,
    TraitType,
    Value,
}

#[derive(Iden)]
enum MetadataCollectionKeys {
    Table,
    MetadataAddress,
    CollectionAddress,
}

/// List query options
#[derive(Debug)]
pub struct ListQueryOptions {
    /// NFT metadata addresses (combines with other filters)
    pub addresses: Option<Vec<String>>,
    /// nft owners
    pub owners: Option<Vec<String>>,
    /// nft update_authorities
    pub update_authorities: Option<Vec<String>>,
    /// auction houses
    pub auction_houses: Option<Vec<String>>,
    /// nft creators
    pub creators: Option<Vec<String>>,
    /// offerers who provided offers on nft
    pub offerers: Option<Vec<String>>,
    /// nft attributes
    pub attributes: Option<Vec<AttributeFilter>>,
    /// nfts listed for sale
    pub listed: Option<bool>,
    /// return nfts from unverified creators
    pub allow_unverified: Option<bool>,
    /// nfts with active offers
    pub with_offers: Option<bool>,
    /// nft in one or more specific collections
    pub collections: Option<Vec<String>>,
    /// limit to apply to query
    pub limit: u64,
    /// offset to apply to query
    pub offset: u64,
}

/// The column set for an NFT
pub type NftColumns = (
    metadatas::address,
    metadatas::name,
    metadatas::seller_fee_basis_points,
    metadatas::mint_address,
    metadatas::primary_sale_happened,
    metadatas::update_authority_address,
    metadatas::uri,
    metadatas::slot,
    metadata_jsons::description,
    metadata_jsons::image,
    metadata_jsons::animation_url,
    metadata_jsons::external_url,
    metadata_jsons::category,
    metadata_jsons::model,
    current_metadata_owners::token_account_address,
);

/// The column set for an NFT
pub const NFT_COLUMNS: NftColumns = (
    metadatas::address,
    metadatas::name,
    metadatas::seller_fee_basis_points,
    metadatas::mint_address,
    metadatas::primary_sale_happened,
    metadatas::update_authority_address,
    metadatas::uri,
    metadatas::slot,
    metadata_jsons::description,
    metadata_jsons::image,
    metadata_jsons::animation_url,
    metadata_jsons::external_url,
    metadata_jsons::category,
    metadata_jsons::model,
    current_metadata_owners::token_account_address,
);

/// Handles queries for NFTs
///
/// # Errors
/// returns an error when the underlying queries throw an error
#[allow(clippy::too_many_lines)]
pub fn list<O: Clone + Into<Value>>(
    conn: &Connection,
    ListQueryOptions {
        addresses,
        owners,
        update_authorities,
        creators,
        auction_houses,
        offerers,
        attributes,
        listed,
        allow_unverified,
        with_offers,
        collections,
        limit,
        offset,
    }: ListQueryOptions,
    opensea_auction_house: O,
) -> Result<Vec<Nft>> {
    let current_time = Utc::now().naive_utc();

    let mut listings_query = Query::select()
        .columns(vec![
            (Listings::Table, Listings::Metadata),
            (Listings::Table, Listings::Price),
            (Listings::Table, Listings::Seller),
        ])
        .from(Listings::Table)
        .order_by((Listings::Table, Listings::Price), Order::Desc)
        .cond_where(
            Condition::all()
                .add(Expr::tbl(Listings::Table, Listings::PurchaseId).is_null())
                .add(Expr::tbl(Listings::Table, Listings::CanceledAt).is_null())
                .add(
                    Expr::tbl(Listings::Table, Listings::AuctionHouse)
                        .ne(opensea_auction_house.clone()),
                )
                .add(
                    Expr::tbl(Listings::Table, Listings::Expiry)
                        .is_null()
                        .or(Expr::tbl(Listings::Table, Listings::Expiry).gt(current_time)),
                ),
        )
        .take();

    if let Some(auction_houses) = auction_houses.clone() {
        listings_query
            .and_where(Expr::col((Listings::Table, Listings::AuctionHouse)).is_in(auction_houses));
    }

    let mut query = Query::select()
        .columns(vec![
            (Metadatas::Table, Metadatas::Address),
            (Metadatas::Table, Metadatas::Name),
            (Metadatas::Table, Metadatas::SellerFeeBasisPoints),
            (Metadatas::Table, Metadatas::UpdateAuthorityAddress),
            (Metadatas::Table, Metadatas::MintAddress),
            (Metadatas::Table, Metadatas::PrimarySaleHappened),
            (Metadatas::Table, Metadatas::Uri),
            (Metadatas::Table, Metadatas::Slot),
        ])
        .columns(vec![
            (MetadataJsons::Table, MetadataJsons::Description),
            (MetadataJsons::Table, MetadataJsons::Image),
            (MetadataJsons::Table, MetadataJsons::AnimationUrl),
            (MetadataJsons::Table, MetadataJsons::ExternalUrl),
            (MetadataJsons::Table, MetadataJsons::Category),
            (MetadataJsons::Table, MetadataJsons::Model),
        ])
        .columns(vec![(
            CurrentMetadataOwners::Table,
            CurrentMetadataOwners::TokenAccountAddress,
        )])
        .from(MetadataJsons::Table)
        .inner_join(
            Metadatas::Table,
            Expr::tbl(MetadataJsons::Table, MetadataJsons::MetadataAddress)
                .equals(Metadatas::Table, Metadatas::Address),
        )
        .inner_join(
            CurrentMetadataOwners::Table,
            Expr::tbl(Metadatas::Table, Metadatas::MintAddress).equals(
                CurrentMetadataOwners::Table,
                CurrentMetadataOwners::MintAddress,
            ),
        )
        .join_lateral(
            JoinType::LeftJoin,
            listings_query.take(),
            Listings::Table,
            Condition::all()
                .add(
                    Expr::tbl(Listings::Table, Listings::Metadata)
                        .equals(Metadatas::Table, Metadatas::Address),
                )
                .add(Expr::tbl(Listings::Table, Listings::Seller).equals(
                    CurrentMetadataOwners::Table,
                    CurrentMetadataOwners::OwnerAddress,
                )),
        )
        .and_where(Expr::col(Metadatas::BurnedAt).is_null())
        .limit(limit)
        .offset(offset)
        .order_by((Listings::Table, Listings::Price), Order::Asc)
        .take();

    if let Some(addresses) = addresses {
        query.and_where(Expr::col(Metadatas::Address).is_in(addresses));
    }

    if let Some(owners) = owners {
        query.and_where(Expr::col(CurrentMetadataOwners::OwnerAddress).is_in(owners));
    }

    if let Some(update_authorities) = update_authorities {
        query.and_where(Expr::col(Metadatas::UpdateAuthorityAddress).is_in(update_authorities));
    }

    if let Some(creators) = creators {
        query
            .inner_join(
                MetadataCreators::Table,
                Expr::tbl(Metadatas::Table, Metadatas::Address)
                    .equals(MetadataCreators::Table, MetadataCreators::MetadataAddress),
            )
            .and_where(Expr::col(MetadataCreators::CreatorAddress).is_in(creators))
            .conditions(
                allow_unverified != Some(true),
                |q| {
                    q.and_where(Expr::col(MetadataCreators::Verified).eq(true));
                },
                |_| {},
            );
    }

    if let Some(listed) = listed {
        query.conditions(
            listed,
            |q| {
                q.and_where(Expr::col((Listings::Table, Listings::Price)).is_not_null());
            },
            |q| {
                q.and_where(Expr::col((Listings::Table, Listings::Price)).is_null());
            },
        );
    }

    let with_offers = with_offers.unwrap_or(false);

    if offerers.is_some() || with_offers {
        let mut offers_conditions = Condition::all().add(
            Expr::tbl(Offers::Table, Offers::Metadata).equals(Metadatas::Table, Metadatas::Address),
        );

        if let Some(offerers) = offerers {
            offers_conditions = offers_conditions
                .add(Expr::col((Offers::Table, Offers::Buyer)).is_in(offerers))
                .add(Expr::tbl(Offers::Table, Offers::PurchaseId).is_null())
                .add(Expr::tbl(Offers::Table, Offers::CanceledAt).is_null())
                .add(Expr::tbl(Offers::Table, Offers::AuctionHouse).ne(opensea_auction_house))
                .add(
                    Expr::tbl(Offers::Table, Offers::Expiry)
                        .is_null()
                        .or(Expr::tbl(Offers::Table, Offers::Expiry).gt(current_time)),
                );
        }

        if with_offers {
            offers_conditions = offers_conditions
                .add(Expr::tbl(Offers::Table, Offers::PurchaseId).is_null())
                .add(Expr::tbl(Offers::Table, Offers::CanceledAt).is_null())
                .add(
                    Expr::tbl(Offers::Table, Offers::Expiry)
                        .is_null()
                        .or(Expr::tbl(Offers::Table, Offers::Expiry).gt(current_time)),
                );
        }

        let mut offers_query = Query::select()
            .columns(vec![
                (Offers::Table, Offers::Metadata),
                (Offers::Table, Offers::Price),
            ])
            .from(Offers::Table)
            .cond_where(offers_conditions)
            .take();

        if let Some(auction_houses) = auction_houses {
            offers_query
                .and_where(Expr::col((Offers::Table, Offers::AuctionHouse)).is_in(auction_houses));
        }

        query.join_lateral(
            JoinType::InnerJoin,
            offers_query.take(),
            Offers::Table,
            Expr::tbl(Offers::Table, Offers::Metadata).equals(Metadatas::Table, Metadatas::Address),
        );
    }

    if let Some(attributes) = attributes {
        for AttributeFilter { trait_type, values } in attributes {
            let alias = format!("attributes_{trait_type}");
            let alias: DynIden = SeaRc::new(Alias::new(&alias));

            query.join_lateral(
                JoinType::InnerJoin,
                Query::select()
                    .from(Attributes::Table)
                    .column((Attributes::Table, Attributes::MetadataAddress))
                    .cond_where(
                        Condition::all()
                            .add(Expr::col(Attributes::TraitType).eq(trait_type))
                            .add(Expr::col(Attributes::Value).is_in(values)),
                    )
                    .take(),
                alias.clone(),
                Expr::tbl(alias, Attributes::MetadataAddress)
                    .equals(Metadatas::Table, Metadatas::Address),
            );
        }
    }

    if let Some(collections) = collections {
        query.inner_join(
            MetadataCollectionKeys::Table,
            Expr::tbl(
                MetadataCollectionKeys::Table,
                MetadataCollectionKeys::MetadataAddress,
            )
            .equals(Metadatas::Table, Metadatas::Address),
        );

        query.and_where(
            Expr::col((
                MetadataCollectionKeys::Table,
                MetadataCollectionKeys::CollectionAddress,
            ))
            .is_in(collections),
        );
    }

    let query = query.to_string(PostgresQueryBuilder);

    diesel::sql_query(query)
        .load(conn)
        .context("Failed to load nft(s)")
}

/// Input parameters for the [`collection_nfts`] query.
#[derive(Debug)]
pub struct CollectionNftOptions {
    /// Collection address
    pub collection: String,
    /// Auction house of the collection
    pub auction_house: Option<String>,
    /// Filter by collection attributes
    pub attributes: Option<Vec<AttributeFilter>>,
    /// Marketplace program in which the collection is listed
    pub marketplace_program: Option<String>,
    /// Sort by Price or Listed at
    pub sort_by: Option<NftSort>,
    /// Order the resulting rows by 'Asc' or 'Desc'
    pub order: Option<Order>,
    /// Limit the number of returned rows
    pub limit: u64,
    /// Skip the first `n` resulting rows
    pub offset: u64,
}

/// Input parameters for the [`wallet_nfts`] query.
#[derive(Debug)]
pub struct WalletNftOptions {
    /// wallet address
    pub wallet: String,
    /// Auction house of the collection
    pub auction_house: Option<String>,
    /// Marketplace program in which the collection is listed
    pub marketplace_program: Option<String>,
    /// nft in one or more specific collections
    pub collections: Option<Vec<String>>,
    /// Sort by Price or Listed at
    pub sort_by: Option<NftSort>,
    /// Order the resulting rows by 'Asc' or 'Desc'
    pub order: Option<Order>,
    /// Limit the number of returned rows
    pub limit: u64,
    /// Skip the first `n` resulting rows
    pub offset: u64,
}

impl From<NftSort> for Listings {
    fn from(sort: NftSort) -> Self {
        match sort {
            NftSort::Price => Listings::Price,
            NftSort::ListedAt => Listings::CreatedAt,
        }
    }
}

/// Handles queries for a Collection Nfts
///
/// # Errors
/// returns an error when the underlying queries throw an error
#[allow(clippy::too_many_lines)]
pub fn collection_nfts<O: Into<Value>>(
    conn: &Connection,
    options: CollectionNftOptions,
    opensea_auction_house: O,
) -> Result<Vec<Nft>> {
    let CollectionNftOptions {
        collection,
        auction_house,
        attributes,
        marketplace_program,
        sort_by,
        order,
        limit,
        offset,
    } = options;

    let sort_by = sort_by.map_or(Listings::Price, Into::into);

    let current_time = Utc::now().naive_utc();

    let order = order.unwrap_or(Order::Desc);
    let uuid = Uuid::parse_str(&collection);
    let is_me_collection = match uuid {
        Err(_error) => false,
        Ok(_result) => true,
    };
    let mut query = Query::select()
        .columns(vec![
            (Metadatas::Table, Metadatas::Address),
            (Metadatas::Table, Metadatas::Name),
            (Metadatas::Table, Metadatas::SellerFeeBasisPoints),
            (Metadatas::Table, Metadatas::UpdateAuthorityAddress),
            (Metadatas::Table, Metadatas::MintAddress),
            (Metadatas::Table, Metadatas::PrimarySaleHappened),
            (Metadatas::Table, Metadatas::Uri),
            (Metadatas::Table, Metadatas::Slot),
        ])
        .columns(vec![
            (MetadataJsons::Table, MetadataJsons::Description),
            (MetadataJsons::Table, MetadataJsons::Image),
            (MetadataJsons::Table, MetadataJsons::AnimationUrl),
            (MetadataJsons::Table, MetadataJsons::ExternalUrl),
            (MetadataJsons::Table, MetadataJsons::Category),
            (MetadataJsons::Table, MetadataJsons::Model),
        ])
        .columns(vec![(
            CurrentMetadataOwners::Table,
            CurrentMetadataOwners::TokenAccountAddress,
        )])
        .from(MetadataJsons::Table)
        .inner_join(
            Metadatas::Table,
            Expr::tbl(MetadataJsons::Table, MetadataJsons::MetadataAddress)
                .equals(Metadatas::Table, Metadatas::Address),
        )
        .inner_join(
            CurrentMetadataOwners::Table,
            Expr::tbl(Metadatas::Table, Metadatas::MintAddress).equals(
                CurrentMetadataOwners::Table,
                CurrentMetadataOwners::MintAddress,
            ),
        )
        .conditions(
            is_me_collection,
            |query| {
                query.inner_join(
                    MeMetadataCollections::Table,
                    Expr::tbl(
                        MeMetadataCollections::Table,
                        MeMetadataCollections::MetadataAddress,
                    )
                    .equals(Metadatas::Table, Metadatas::Address),
                );
            },
            |query| {
                query.inner_join(
                    MetadataCollectionKeys::Table,
                    Expr::tbl(
                        MetadataCollectionKeys::Table,
                        MetadataCollectionKeys::MetadataAddress,
                    )
                    .equals(Metadatas::Table, Metadatas::Address),
                );
            },
        )
        .left_join(
            Listings::Table,
            Condition::all()
                .add(
                    Expr::tbl(Listings::Table, Listings::Metadata)
                        .equals(Metadatas::Table, Metadatas::Address),
                )
                .add(Expr::tbl(Listings::Table, Listings::Seller).equals(
                    CurrentMetadataOwners::Table,
                    CurrentMetadataOwners::OwnerAddress,
                ))
                .add(Expr::tbl(Listings::Table, Listings::PurchaseId).is_null())
                .add(Expr::tbl(Listings::Table, Listings::CanceledAt).is_null())
                .add(Expr::tbl(Listings::Table, Listings::AuctionHouse).ne(opensea_auction_house))
                .add(
                    Expr::tbl(Listings::Table, Listings::Expiry)
                        .is_null()
                        .or(Expr::tbl(Listings::Table, Listings::Expiry).gt(current_time)),
                )
                .add_option(auction_house.map(|auction_house| {
                    Expr::col((Listings::Table, Listings::AuctionHouse)).eq(auction_house)
                }))
                .add_option(marketplace_program.map(|marketplace_program| {
                    Expr::col((Listings::Table, Listings::MarketplaceProgram))
                        .eq(marketplace_program)
                })),
        )
        .and_where(Expr::col(Metadatas::BurnedAt).is_null())
        .conditions(
            is_me_collection,
            |query| {
                query.and_where(
                    Expr::col((
                        MeMetadataCollections::Table,
                        MeMetadataCollections::CollectionId,
                    ))
                    .eq(collection.clone()),
                );
            },
            |query| {
                query.and_where(
                    Expr::col((
                        MetadataCollectionKeys::Table,
                        MetadataCollectionKeys::CollectionAddress,
                    ))
                    .eq(collection.clone()),
                );
            },
        )
        .limit(limit)
        .offset(offset)
        .order_by((Listings::Table, sort_by), order)
        .take();

    if let Some(attributes) = attributes {
        for AttributeFilter { trait_type, values } in attributes {
            let alias = format!("attributes_{trait_type}");
            let alias: DynIden = SeaRc::new(Alias::new(&alias));

            query.join_lateral(
                JoinType::LeftJoin,
                Query::select()
                    .from(Attributes::Table)
                    .column((Attributes::Table, Attributes::MetadataAddress))
                    .cond_where(
                        Condition::all()
                            .add(Expr::col(Attributes::TraitType).eq(trait_type))
                            .add(Expr::col(Attributes::Value).is_in(values)),
                    )
                    .take(),
                alias.clone(),
                Expr::tbl(alias, Attributes::MetadataAddress)
                    .equals(Metadatas::Table, Metadatas::Address),
            );
        }
    }

    let query = query.to_string(PostgresQueryBuilder);

    diesel::sql_query(query)
        .load(conn)
        .context("Failed to load nft(s)")
}

/// Handles queries for a wallet Nfts
///
/// # Errors
/// returns an error when the underlying queries throw an error
#[allow(clippy::too_many_lines)]
pub fn wallet_nfts<O: Into<Value>>(
    conn: &Connection,
    options: WalletNftOptions,
    opensea_auction_house: O,
) -> Result<Vec<Nft>> {
    let WalletNftOptions {
        wallet,
        auction_house,
        marketplace_program,
        collections,
        sort_by,
        order,
        limit,
        offset,
    } = options;

    let sort_unwrap = sort_by.map_or(Listings::Price, Into::into);

    let order_unwrap = order.unwrap_or(Order::Desc);

    let current_time = Utc::now().naive_utc();

    let mut query = Query::select()
        .columns(vec![
            (Metadatas::Table, Metadatas::Address),
            (Metadatas::Table, Metadatas::Name),
            (Metadatas::Table, Metadatas::SellerFeeBasisPoints),
            (Metadatas::Table, Metadatas::UpdateAuthorityAddress),
            (Metadatas::Table, Metadatas::MintAddress),
            (Metadatas::Table, Metadatas::PrimarySaleHappened),
            (Metadatas::Table, Metadatas::Uri),
            (Metadatas::Table, Metadatas::Slot),
        ])
        .columns(vec![
            (MetadataJsons::Table, MetadataJsons::Description),
            (MetadataJsons::Table, MetadataJsons::Image),
            (MetadataJsons::Table, MetadataJsons::AnimationUrl),
            (MetadataJsons::Table, MetadataJsons::ExternalUrl),
            (MetadataJsons::Table, MetadataJsons::Category),
            (MetadataJsons::Table, MetadataJsons::Model),
        ])
        .columns(vec![(
            CurrentMetadataOwners::Table,
            CurrentMetadataOwners::TokenAccountAddress,
        )])
        .columns(vec![
            (Listings::Table, Listings::Metadata),
            (Listings::Table, Listings::Price),
            (Listings::Table, Listings::Seller),
        ])
        .from(MetadataJsons::Table)
        .inner_join(
            Metadatas::Table,
            Expr::tbl(MetadataJsons::Table, MetadataJsons::MetadataAddress)
                .equals(Metadatas::Table, Metadatas::Address),
        )
        .inner_join(
            CurrentMetadataOwners::Table,
            Expr::tbl(Metadatas::Table, Metadatas::MintAddress).equals(
                CurrentMetadataOwners::Table,
                CurrentMetadataOwners::MintAddress,
            ),
        )
        .left_join(
            Listings::Table,
            Condition::all()
                .add(
                    Expr::tbl(Listings::Table, Listings::Metadata)
                        .equals(Metadatas::Table, Metadatas::Address),
                )
                .add(Expr::tbl(Listings::Table, Listings::Seller).equals(
                    CurrentMetadataOwners::Table,
                    CurrentMetadataOwners::OwnerAddress,
                ))
                .add(Expr::tbl(Listings::Table, Listings::PurchaseId).is_null())
                .add(Expr::tbl(Listings::Table, Listings::CanceledAt).is_null())
                .add(Expr::tbl(Listings::Table, Listings::AuctionHouse).ne(opensea_auction_house))
                .add(
                    Expr::tbl(Listings::Table, Listings::Expiry)
                        .is_null()
                        .or(Expr::tbl(Listings::Table, Listings::Expiry).gt(current_time)),
                )
                .add_option(auction_house.map(|auction_house| {
                    Expr::col((Listings::Table, Listings::AuctionHouse)).eq(auction_house)
                }))
                .add_option(marketplace_program.map(|marketplace_program| {
                    Expr::col((Listings::Table, Listings::MarketplaceProgram))
                        .eq(marketplace_program)
                })),
        )
        .cond_where(
            Condition::all()
                .add(Expr::col(CurrentMetadataOwners::OwnerAddress).eq(wallet))
                .add(Expr::col(Metadatas::BurnedAt).is_null()),
        )
        .limit(limit)
        .offset(offset)
        .order_by((Listings::Table, sort_unwrap), order_unwrap)
        .take();

    if let Some(collections) = collections {
        query.inner_join(
            CollectionMints::Table,
            Expr::tbl(CollectionMints::Table, CollectionMints::Mint)
                .equals(Metadatas::Table, Metadatas::MintAddress),
        );

        query.and_where(
            Expr::col((CollectionMints::Table, CollectionMints::CollectionId)).is_in(collections),
        );
    }

    let query = query.to_string(PostgresQueryBuilder);

    diesel::sql_query(query)
        .load(conn)
        .context("Failed to load wallet nft(s)")
}

const ACTIVITES_QUERY: &str = r"
SELECT listings.id as id, metadata, auction_house, price, auction_house, created_at, marketplace_program,
    array[seller] as wallets,
    array[twitter_handle_name_services.twitter_handle] as wallet_twitter_handles,
    'listing' as activity_type
        FROM listings
        LEFT JOIN twitter_handle_name_services on (twitter_handle_name_services.wallet_address = listings.seller)
        WHERE metadata = ANY($1) and auction_house != '3o9d13qUvEuuauhFrVom1vuCzgNsJifeaBYDPquaT73Y'
    UNION
    SELECT purchases.id as id, metadata, auction_house, price, auction_house, created_at, marketplace_program,
    array[seller, buyer] as wallets,
    array[sth.twitter_handle, bth.twitter_handle] as wallet_twitter_handles,
    'purchase' as activity_type
        FROM purchases
        LEFT JOIN twitter_handle_name_services sth on (sth.wallet_address = purchases.seller)
        LEFT JOIN twitter_handle_name_services bth on (bth.wallet_address = purchases.buyer)
        WHERE metadata = ANY($1)
    UNION
    SELECT offers.id as id, metadata, auction_house, price, auction_house, created_at, marketplace_program,
    array[buyer] as wallets,
    array[bth.twitter_handle] as wallet_twitter_handles,
    'offer' as activity_type
        FROM offers
        LEFT JOIN twitter_handle_name_services bth on (bth.wallet_address = offers.buyer)
        WHERE metadata = ANY($1) and auction_house != '3o9d13qUvEuuauhFrVom1vuCzgNsJifeaBYDPquaT73Y'
        AND offers.purchase_id IS NULL
    ORDER BY created_at DESC;
 -- $1: addresses::text[]";

/// Load listing and sales activity for nfts
///
/// # Errors
/// This function fails if the underlying SQL query returns an error
pub fn activities(
    conn: &Connection,
    addresses: impl ToSql<Array<Text>, Pg>,
) -> Result<Vec<NftActivity>> {
    diesel::sql_query(ACTIVITES_QUERY)
        .bind(addresses)
        .load(conn)
        .context("Failed to load nft(s) activities")
}

/// Handles queries for a Moonrank Collection Nfts
///
/// # Errors
/// returns an error when the underlying queries throw an error
#[allow(clippy::too_many_lines)]
pub fn mr_collection_nfts<O: Into<Value>>(
    conn: &Connection,
    options: CollectionNftOptions,
    opensea_auction_house: O,
) -> Result<Vec<Nft>> {
    let CollectionNftOptions {
        collection,
        auction_house,
        attributes,
        marketplace_program,
        sort_by,
        order,
        limit,
        offset,
    } = options;

    let sort_by = sort_by.map_or(Listings::Price, Into::into);

    let current_time = Utc::now().naive_utc();

    let order = order.unwrap_or(Order::Desc);

    let mut query = Query::select()
        .columns(vec![
            (Metadatas::Table, Metadatas::Address),
            (Metadatas::Table, Metadatas::Name),
            (Metadatas::Table, Metadatas::SellerFeeBasisPoints),
            (Metadatas::Table, Metadatas::UpdateAuthorityAddress),
            (Metadatas::Table, Metadatas::MintAddress),
            (Metadatas::Table, Metadatas::PrimarySaleHappened),
            (Metadatas::Table, Metadatas::Uri),
            (Metadatas::Table, Metadatas::Slot),
        ])
        .columns(vec![
            (MetadataJsons::Table, MetadataJsons::Description),
            (MetadataJsons::Table, MetadataJsons::Image),
            (MetadataJsons::Table, MetadataJsons::AnimationUrl),
            (MetadataJsons::Table, MetadataJsons::ExternalUrl),
            (MetadataJsons::Table, MetadataJsons::Category),
            (MetadataJsons::Table, MetadataJsons::Model),
        ])
        .columns(vec![(
            CurrentMetadataOwners::Table,
            CurrentMetadataOwners::TokenAccountAddress,
        )])
        .from(MetadataJsons::Table)
        .inner_join(
            Metadatas::Table,
            Expr::tbl(MetadataJsons::Table, MetadataJsons::MetadataAddress)
                .equals(Metadatas::Table, Metadatas::Address),
        )
        .inner_join(
            CurrentMetadataOwners::Table,
            Expr::tbl(Metadatas::Table, Metadatas::MintAddress).equals(
                CurrentMetadataOwners::Table,
                CurrentMetadataOwners::MintAddress,
            ),
        )
        .inner_join(
            CollectionMints::Table,
            Expr::tbl(CollectionMints::Table, CollectionMints::Mint)
                .equals(Metadatas::Table, Metadatas::MintAddress),
        )
        .left_join(
            Listings::Table,
            Condition::all()
                .add(
                    Expr::tbl(Listings::Table, Listings::Metadata)
                        .equals(Metadatas::Table, Metadatas::Address),
                )
                .add(Expr::tbl(Listings::Table, Listings::Seller).equals(
                    CurrentMetadataOwners::Table,
                    CurrentMetadataOwners::OwnerAddress,
                ))
                .add(Expr::tbl(Listings::Table, Listings::PurchaseId).is_null())
                .add(Expr::tbl(Listings::Table, Listings::CanceledAt).is_null())
                .add(Expr::tbl(Listings::Table, Listings::AuctionHouse).ne(opensea_auction_house))
                .add(
                    Expr::tbl(Listings::Table, Listings::Expiry)
                        .is_null()
                        .or(Expr::tbl(Listings::Table, Listings::Expiry).gt(current_time)),
                )
                .add_option(auction_house.map(|auction_house| {
                    Expr::col((Listings::Table, Listings::AuctionHouse)).eq(auction_house)
                }))
                .add_option(marketplace_program.map(|marketplace_program| {
                    Expr::col((Listings::Table, Listings::MarketplaceProgram))
                        .eq(marketplace_program)
                })),
        )
        .and_where(Expr::col(Metadatas::BurnedAt).is_null())
        .and_where(
            Expr::col((CollectionMints::Table, CollectionMints::CollectionId)).eq(collection),
        )
        .limit(limit)
        .offset(offset)
        .order_by((Listings::Table, sort_by), order)
        .take();

    if let Some(attributes) = attributes {
        for AttributeFilter { trait_type, values } in attributes {
            let alias = format!("attributes_{trait_type}");
            let alias: DynIden = SeaRc::new(Alias::new(&alias));

            query
                .join_as(
                    JoinType::InnerJoin,
                    Attributes::Table,
                    alias.clone(),
                    Expr::tbl(alias.clone(), Attributes::MetadataAddress)
                        .equals(Metadatas::Table, Metadatas::Address),
                )
                .cond_where(
                    Condition::all()
                        .add(Expr::col((alias.clone(), Attributes::TraitType)).eq(trait_type))
                        .add(Expr::col((alias, Attributes::Value)).is_in(values)),
                );
        }
    }

    let query = query.to_string(PostgresQueryBuilder);

    diesel::sql_query(query)
        .load(conn)
        .context("Failed to load nft(s)")
}
