import { Buffer } from "buffer";
import { Address } from "@stellar/stellar-sdk";
import {
  AssembledTransaction,
  Client as ContractClient,
  ClientOptions as ContractClientOptions,
  MethodOptions,
  Result,
  Spec as ContractSpec,
} from "@stellar/stellar-sdk/contract";
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Timepoint,
  Duration,
} from "@stellar/stellar-sdk/contract";
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";

if (typeof window !== "undefined") {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}






/**
 * NFT data structure stored on-chain.
 */
export interface NftData {
  completion_player: string;
  hunt_id: u64;
  metadata: NftMetadata;
  minted_at: u64;
  nft_id: u64;
  owner: string;
  transferable: boolean;
}


/**
 * Collection-level metadata for the NFT reward contract (matches the
 * on-chain `CollectionMetadata` struct used by `initialize`).
 */
export interface CollectionMetadata {
  creator: Option<string>;
  description: string;
  name: string;
  total_supply: u64;
}


/**
 * Core display metadata for an NFT (title, description, image URI).
 * Supports off-chain storage references to keep gas costs low.
 */
export interface NftMetadata {
  /**
 * Original creator of the NFT (stamped at mint time for provenance/attribution).
 * Essential for secondary market royalty distribution and creator attribution.
 */
 creator: Option<string>;
  description: string;
  /**
 * Hunt title at time of mint (for context/display).
 */
 hunt_title: string;
 image_uri: string;
  /**
 * Rarity tier: 0 = default, 1 = common, 2 = uncommon, 3 = rare, 4 = epic, 5 = legendary.
 */
 rarity: u32;
  /**
 * Royalty in basis points (1 bp = 0.01%). For example, 250 = 2.5% royalty.
 * Used for secondary market sales to provide ongoing creator revenue.
 */
 royalty_bps: Option<u32>;
 /**
 * Custom tier for special categories (0 = none).
 */
 tier: u32;
 title: string;
}


/**
 * Event emitted when an NFT is minted.
 */
export interface NftMintedEvent {
  hunt_id: u64;
  metadata: NftMetadata;
  minted_at: u64;
  nft_id: u64;
  owner: string;
  rarity: u32;
  tier: u32;
}


/**
 * Complete metadata returned by get_nft_metadata (includes NftData-derived fields).
 */
export interface NftMetadataResponse {
  completion_player: string;
  completion_timestamp: u64;
  creator: Option<string>;
  current_owner: string;
  description: string;
  hunt_id: u64;
  hunt_title: string;
  image_uri: string;
  nft_id: u64;
  rarity: u32;
  royalty_bps: Option<u32>;
  schema_version: u32;
  tier: u32;
  title: string;
}


/**
 * Event emitted when an NFT is transferred.
 */
export interface NftTransferredEvent {
  from: string;
  nft_id: u64;
  to: string;
}


/**
 * Event emitted when an owner changes operator approval.
 */
export interface OperatorChangedEvent {
  approved: boolean;
  operator: string;
  owner: string;
}


/**
 * Event emitted when an NFT's mutable metadata is updated.
 */
export interface NftMetadataUpdatedEvent {
  nft_id: u64;
  updater: string;
}


/**
 * Event emitted when admin batch-updates image URIs across NFTs.
 */
export interface AdminImageUrisUpdatedEvent {
  new_prefix: string;
  old_prefix: string;
  updated_count: u32;
}

export const NftErrorCode = {
  3001: {message:"NftNotFound"},
  3002: {message:"Unauthorized"},
  3003: {message:"NotOwner"},
  3004: {message:"InvalidRecipient"},
  3005: {message:"SoulboundNft"},
  3006: {message:"InvalidRarity"},
  3007: {message:"AlreadyInitialized"},
  3008: {message:"MaxSupplyReached"},
  3009: {message:"NotInitialized"},
  3010: {message:"NotOperator"},
  3011: {message:"NftNotTransferable"},
  3012: {message:"NftLocked"},
  3013: {message:"InvalidMetadata"},
  3014: {message:"MetadataFrozen"},
  3015: {message:"TooManyExtensions"},
  3016: {message:"InvalidExtensionKey"},
  3017: {message:"InvalidExtensionValue"},
  3018: {message:"ExtensionNotFound"},
  3019: {message:"InvalidMaxSupply"}
}


export interface MigrationReport {
  dry_run: boolean;
  from_version: u32;
  message: string;
  steps_applied: u32;
  succeeded: boolean;
  to_version: u32;
}

export interface Client {
  /**
   * Construct and simulate a burn transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Burns (permanently destroys) an NFT, removing it from storage and the owner's list.
   * 
   * # Authorization
   * The `owner` must authorize this call. The caller must also be the current owner.
   */
  burn: ({nft_id, owner}: {nft_id: u64, owner: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>

  /**
   * Construct and simulate a get_nft transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Retrieves NFT data by ID.
   */
  get_nft: ({nft_id}: {nft_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<Option<NftData>>>

  /**
   * Construct and simulate a owner_of transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the owner of an NFT.
   */
  owner_of: ({nft_id}: {nft_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<Option<string>>>

  /**
   * Construct and simulate a get_admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the configured admin address, if set.
   */
  get_admin: (options?: MethodOptions) => Promise<AssembledTransaction<Option<string>>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Initializes the NFT reward contract with an admin address and optional max supply cap.
   * Call this once to set the admin who can manage the contract.
   */
  initialize: ({_admin, _minter, _max_supply, _metadata}: {_admin: string, _minter: string, _max_supply: Option<u64>, _metadata: CollectionMetadata}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate an is_operator transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns true if `operator` is approved to manage all NFTs of `owner`.
   */
  is_operator: ({owner, operator}: {owner: string, operator: string}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a has_hunt_nft transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns `true` if `address` owns any NFT minted for `hunt_id`.
   * Scans the owner's indexed NFT IDs and checks each NFT's `hunt_id`.
   */
  has_hunt_nft: ({address, hunt_id}: {address: string, hunt_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a set_operator transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Grants `operator` the ability to manage all NFTs owned by `owner`.
   * 
   * # Authorization
   * `owner` must authorize this call.
   */
  set_operator: ({owner, operator}: {owner: string, operator: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a total_supply transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the total number of NFTs minted so far.
   */
  total_supply: (options?: MethodOptions) => Promise<AssembledTransaction<u64>>

  /**
   * Construct and simulate a transfer_nft transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Transfers an NFT from one address to another.
   * 
   * # Arguments
   * * `nft_id` - The NFT to transfer
   * * `from_address` - Current owner of the NFT
   * * `to_address` - New owner
   * * `caller` - Address authorizing the transfer (must be owner or approved operator)
   * 
   * # Authorization
   * `caller` must authorize this call. `caller` must be either the current owner
   * or an operator approved by the owner via `set_operator`.
   */
  transfer_nft: ({nft_id, from_address, to_address, caller}: {nft_id: u64, from_address: string, to_address: string, caller: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a run_migration transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  run_migration: ({admin, target_version, dry_run}: {admin: string, target_version: u32, dry_run: boolean}, options?: MethodOptions) => Promise<AssembledTransaction<MigrationReport>>

  /**
   * Construct and simulate a search_by_tier transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Searches NFTs by tier.
   */
  search_by_tier: ({tier}: {tier: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Array<u64>>>

  /**
   * Construct and simulate a get_player_nfts transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns paginated NFT IDs owned by an address.
   */
  get_player_nfts: ({owner, offset, limit}: {owner: string, offset: u32, limit: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Array<u64>>>

  /**
   * Construct and simulate a mint_reward_nft transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Mints a unique NFT as a reward for hunt completion.
   * 
   * `minter` must be an authorized minter (and must sign the transaction) when the
   * contract has been initialized.  Before initialization the check is skipped so
   * that existing deployments remain functional.
   * 
   * # Arguments
   * * `minter` - Address performing the mint (must be whitelisted after init)
   * * `hunt_id` - The hunt this NFT commemorates
   * * `player_address` - The address of the player completing the hunt (initial owner)
   * * `metadata` - NFT metadata (title, description, image URI, hunt_title, rarity, tier)
   * 
   * # Returns
   * The unique NFT ID of the minted NFT
   */
  mint_reward_nft: ({_minter, _hunt_id, _owner, metadata}: {_minter: string, _hunt_id: u64, _owner: string, metadata: NftMetadata}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u64>>>

  /**
   * Construct and simulate a search_nfts_by_metadata transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Searches NFTs matching metadata criteria with a bounded scan and cursor.
   * 
   * Scans at most `MAX_SCAN_LIMIT` NFT records starting from `cursor` and
   * returns the matching IDs along with the next cursor (or `None` when the
   * scan is complete). Callers paginate by passing the returned cursor back in
   * until it is `none`.
   */
  search_nfts_by_metadata: ({title_contains, creator, cursor, limit}: {title_contains: Option<string>, creator: Option<string>, cursor: Option<u64>, limit: u32}, options?: MethodOptions) => Promise<AssembledTransaction<MetadataSearchResult>>
}

export interface MetadataSearchResult {
  /**
   * Matching NFT IDs for this page.
   */
  nft_ids: Array<u64>;
  /**
   * Cursor to pass into the next call, or `none` when the scan is complete.
   */
  next_cursor: Option<u64>;
  /**
   * Number of NFT records actually scanned in this call.
   */
  scanned_count: u32;
}

export const Networks = {
  "testnet": {
    "networkPassphrase": "Test SDT Network ; February 2019",
    "contractId": ""
  },
  "futurenet": {
    "networkPassphrase": "Test SDF Futurenet ; September 2019",
    "contractId": ""
  },
  "mainnet": {
    "networkPassphrase": "Public Global Network ; September 2015",
    "contractId": ""
  }
}

export class Client extends ContractClient {
  constructor(options: ContractClientOptions = { contractId: '' }) {
    super(new ContractSpec([{
      "type": "function",
      "name": "burn",
      "inputs": [
        {
          "name": "nft_id",
          "type": "u64"
        },
        {
          "name": "owner",
          "type": "address"
        }
      ],
      "outputs": [
        {
          "type": "result"
        }
      ]
    },
    {
      "type": "function",
      "name": "get_nft",
      "inputs": [
        {
          "name": "nft_id",
          "type": "u64"
        }
      ],
      "outputs": [
        {
          "type": "option"
        }
      ]
    },
    {
      "type": "function",
      "name": "owner_of",
      "inputs": [
        {
          "name": "nft_id",
          "type": "u64"
        }
      ],
      "outputs": [
        {
          "type": "option"
        }
      ]
    },
    {
      "type": "function",
      "name": "get_admin",
      "inputs": [],
      "outputs": [
        {
          "type": "option"
        }
      ]
    },
    {
      "type": "function",
      "name": "is_operator",
      "inputs": [
        {
          "name": "owner",
          "type": "address"
        },
        {
          "name": "operator",
          "type": "address"
        }
      ],
      "outputs": [
        {
          "type": "boolean"
        }
      ]
    },
    {
      "type": "function",
      "name": "has_hunt_nft",
      "inputs": [
        {
          "name": "address",
          "type": "address"
        },
        {
          "name": "hunt_id",
          "type": "u64"
        }
      ],
      "outputs": [
        {
          "type": "boolean"
        }
      ]
    },
    {
      "type": "function",
      "name": "set_operator",
      "inputs": [
        {
          "name": "owner",
          "type": "address"
        },
        {
          "name": "operator",
          "type": "address"
        }
      ],
      "outputs": []
    },
    {
      "type": "function",
      "name": "total_supply",
      "inputs": [],
      "outputs": [
        {
          "type": "u64"
        }
      ]
    },
    {
      "type": "function",
      "name": "transfer_nft",
      "inputs": [
        {
          "name": "nft_id",
          "type": "u64"
        },
        {
          "name": "from_address",
          "type": "address"
        },
        {
          "name": "to_address",
          "type": "address"
        },
        {
          "name": "caller",
          "type": "address"
        }
      ],
      "outputs": [
        {
          "type": "result"
        }
      ]
    },
    {
      "type": "function",
      "name": "run_migration",
      "inputs": [
        {
          "name": "admin",
          "type": "address"
        },
        {
          "name": "target_version",
          "type": "u32"
        },
        {
          "name": "dry_run",
          "type": "boolean"
        }
      ],
      "outputs": [
        {
          "type": "defined",
          "name": "MigrationReport"
        }
      ]
    },
    {
      "type": "function",
      "name": "search_by_tier",
      "inputs": [
        {
          "name": "tier",
          "type": "u32"
        }
      ],
      "outputs": [
        {
          "type": "array"
        }
      ]
    },
    {
      "type": "function",
      "name": "get_player_nfts",
      "inputs": [
        {
          "name": "owner",
          "type": "address"
        },
        {
          "name": "offset",
          "type": "u32"
        },
        {
          "name": "limit",
          "type": "u32"
        }
      ],
      "outputs": [
        {
          "type": "array"
        }
      ]
    },
    {
      "type": "function",
      "name": "mint_reward_nft",
      "inputs": [
        {
          "name": "_minter",
          "type": "address"
        },
        {
          "name": "hunt_id",
          "type": "u64"
        },
        {
          "name": "player_address",
          "type": "address"
        },
        {
          "name": "metadata",
          "type": "defined",
          "name": "NftMetadata"
        }
      ],
      "outputs": [
        {
          "type": "result"
        }
      ]
    },
    {
      "type": "function",
      "name": "search_nfts_by_metadata",
      "inputs": [
        {
          "name": "title_contains",
          "type": "option"
        },
        {
          "name": "creator",
          "type": "option"
        },
        {
          "name": "cursor",
          "type": "option"
        },
        {
          "name": "limit",
          "type": "u32"
        }
      ],
      "outputs": [
        {
          "type": "defined",
          "name": "MetadataSearchResult"
        }
      ]
    },
    {
      "type": "defined",
      "name": "MetadataSearchResult",
      "fields": [
        {
          "name": "nft_ids",
          "type": "array"
        },
        {
          "name": "next_cursor",
          "type": "option"
        },
        {
          "name": "scanned_count",
          "type": "u32"
        }
      ]
    },
    {
      "type": "defined",
      "name": "MigrationReport",
      "fields": [
        {
          "name": "dry_run",
          "type": "boolean"
        },
        {
          "name": "from_version",
          "type": "u32"
        },
        {
          "name": "message",
          "type": "string"
        },
        {
          "name": "steps_applied",
          "type": "u32"
        },
        {
          "name": "succeeded",
          "type": "boolean"
        },
        {
          "name": "to_version",
          "type": "u32"
        }
      ]
    },
    {
      "type": "defined",
      "name": "NftMetadata",
      "fields": [
        {
          "name": "creator",
          "type": "option"
        },
        {
          "name": "description",
          "type": "string"
        },
        {
          "name": "hunt_title",
          "type": "string"
        },
        {
          "name": "image_uri",
          "type": "string"
        },
        {
          "name": "rarity",
          "type": "u32"
        },
        {
          "name": "royalty_bps",
          "type": "option"
        },
        {
          "name": "tier",
          "type": "u32"
        },
        {
          "name": "title",
          "type": "string"
        }
      ]
    }
  ]), options)
  }
}
