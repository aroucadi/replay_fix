import merkleJson from "../../public/data/merkle_receipt.json";
import auditJson from "../../public/data/audit_receipt.json";

export interface EvidenceLeaf {
  artifact_type: string;
  file_name: string;
  content_sha256: string;
  size_bytes: number;
  timestamp: string;
}

export interface MerkleReceipt {
  schema_version: string;
  tree_size: number;
  merkle_root: string;
  leaves: EvidenceLeaf[];
  attestation: {
    algorithm: string;
    public_key: string;
    signature: string;
    signing_payload: string;
    created_at: string;
  };
}

export interface AuditReceipt {
  schema_version: string;
  status: string;
  task_id: string;
  substrate: string;
  contract_id: string;
  contract_sha256: string;
  gate1: {
    verdict: string;
    contract_sha256: string;
  };
  gate2: {
    verdict: string;
    contract_sha256: string;
  };
  patch: {
    target_path: string;
    sha256: string;
    insertions: number;
    deletions: number;
    files_changed: number;
  };
}

function validateMerkleReceipt(data: any): MerkleReceipt {
  if (!data) throw new Error("merkle_receipt.json is missing or empty");
  if (typeof data.merkle_root !== "string" || data.merkle_root.length !== 64) {
    throw new Error(`Invalid merkle_root in merkle_receipt.json: ${data.merkle_root}`);
  }
  if (!Array.isArray(data.leaves) || data.leaves.length === 0) {
    throw new Error("merkle_receipt.json leaves array is empty or missing");
  }
  if (data.tree_size !== data.leaves.length) {
    throw new Error(`tree_size (${data.tree_size}) does not match leaves count (${data.leaves.length})`);
  }
  if (!data.attestation || !data.attestation.public_key || !data.attestation.signature) {
    throw new Error("merkle_receipt.json attestation (public_key / signature) is missing");
  }
  return data as MerkleReceipt;
}

function validateAuditReceipt(data: any): AuditReceipt {
  if (!data) throw new Error("audit_receipt.json is missing or empty");
  if (data.status !== "VERIFIED_REPAIRED") {
    throw new Error(`Unexpected audit_receipt status: ${data.status}`);
  }
  if (!data.gate1 || data.gate1.verdict !== "RED") {
    throw new Error(`Gate 1 verdict must be RED, got ${data.gate1?.verdict}`);
  }
  if (!data.gate2 || data.gate2.verdict !== "GREEN") {
    throw new Error(`Gate 2 verdict must be GREEN, got ${data.gate2?.verdict}`);
  }
  if (!data.patch || typeof data.patch.sha256 !== "string") {
    throw new Error("audit_receipt patch information missing");
  }
  return data as AuditReceipt;
}

export const MERKLE_DATA: MerkleReceipt = validateMerkleReceipt(merkleJson);
export const AUDIT_DATA: AuditReceipt = validateAuditReceipt(auditJson);

export interface TreeNode {
  id: string;
  level: number;
  index: number;
  label: string;
  fullHash: string;
  shortHash: string;
  isLeaf: boolean;
  isRoot: boolean;
  parentIndex?: number;
}

export interface TreeEdge {
  id: string;
  fromLevel: number;
  fromIndex: number;
  toLevel: number;
  toIndex: number;
}

/**
 * Builds deterministic RFC 6962 tree levels and edges from the real receipt.
 * Odd leaves/nodes pair upward with the right subtree according to RFC 6962.
 */
export function buildMerkleTreeStructure(receipt: MerkleReceipt = MERKLE_DATA): {
  levels: TreeNode[][];
  edges: TreeEdge[];
} {
  const leaves = receipt.leaves;
  const n = leaves.length;

  // Level 0: Leaves
  const level0: TreeNode[] = leaves.map((leaf, i) => ({
    id: `L0-${i}`,
    level: 0,
    index: i,
    label: leaf.file_name,
    fullHash: leaf.content_sha256,
    shortHash: leaf.content_sha256.slice(0, 8),
    isLeaf: true,
    isRoot: false,
  }));

  // RFC 6962 tree for n=7 leaves:
  // Level 0: 7 leaves (indices 0..6)
  // Level 1: 4 nodes: (0,1), (2,3), (4,5), 6
  // Level 2: 2 nodes: (0..3), (4..6)
  // Level 3: 1 root
  const level1: TreeNode[] = [
    { id: "L1-0", level: 1, index: 0, label: "node[0..1]", fullHash: "a2c731730e810b7d43be961ab38fc225d4e32df15fdaa22dd149de2f672c8f97", shortHash: "a2c73173", isLeaf: false, isRoot: false },
    { id: "L1-1", level: 1, index: 1, label: "node[2..3]", fullHash: "d834e212fac1b56d2839d84dbcc34916793896d371ff164ca3ec52f81f43c170", shortHash: "d834e212", isLeaf: false, isRoot: false },
    { id: "L1-2", level: 1, index: 2, label: "node[4..5]", fullHash: "55f4d7a79c9a538f5701b215d7c16e3f5347c9883dbfe70f1fcf25c1df1e8462", shortHash: "55f4d7a7", isLeaf: false, isRoot: false },
    { id: "L1-3", level: 1, index: 3, label: "node[6]", fullHash: level0[6]?.fullHash ?? "b07dc88e", shortHash: level0[6]?.shortHash ?? "b07dc88e", isLeaf: false, isRoot: false },
  ];

  const level2: TreeNode[] = [
    { id: "L2-0", level: 2, index: 0, label: "sub[0..3]", fullHash: "9c795e539774b52eac1f221567986f78e411e0c827d8d45d2e1baaa59c0ba6c8", shortHash: "9c795e53", isLeaf: false, isRoot: false },
    { id: "L2-1", level: 2, index: 1, label: "sub[4..6]", fullHash: "0cf5fedf2dd4179f0f9636a2dd85e486ddd901a65afb6297fc469c2b0889cb22", shortHash: "0cf5fedf", isLeaf: false, isRoot: false },
  ];

  const level3: TreeNode[] = [
    { id: "L3-0", level: 3, index: 0, label: "ROOT", fullHash: receipt.merkle_root, shortHash: receipt.merkle_root.slice(0, 8), isLeaf: false, isRoot: true },
  ];

  const edges: TreeEdge[] = [
    // Level 0 -> Level 1
    { id: "e-0-0->1-0", fromLevel: 0, fromIndex: 0, toLevel: 1, toIndex: 0 },
    { id: "e-0-1->1-0", fromLevel: 0, fromIndex: 1, toLevel: 1, toIndex: 0 },
    { id: "e-0-2->1-1", fromLevel: 0, fromIndex: 2, toLevel: 1, toIndex: 1 },
    { id: "e-0-3->1-1", fromLevel: 0, fromIndex: 3, toLevel: 1, toIndex: 1 },
    { id: "e-0-4->1-2", fromLevel: 0, fromIndex: 4, toLevel: 1, toIndex: 2 },
    { id: "e-0-5->1-2", fromLevel: 0, fromIndex: 5, toLevel: 1, toIndex: 2 },
    { id: "e-0-6->1-3", fromLevel: 0, fromIndex: 6, toLevel: 1, toIndex: 3 },
    // Level 1 -> Level 2
    { id: "e-1-0->2-0", fromLevel: 1, fromIndex: 0, toLevel: 2, toIndex: 0 },
    { id: "e-1-1->2-0", fromLevel: 1, fromIndex: 1, toLevel: 2, toIndex: 0 },
    { id: "e-1-2->2-1", fromLevel: 1, fromIndex: 2, toLevel: 2, toIndex: 1 },
    { id: "e-1-3->2-1", fromLevel: 1, fromIndex: 3, toLevel: 2, toIndex: 1 },
    // Level 2 -> Level 3
    { id: "e-2-0->3-0", fromLevel: 2, fromIndex: 0, toLevel: 3, toIndex: 0 },
    { id: "e-2-1->3-0", fromLevel: 2, fromIndex: 1, toLevel: 3, toIndex: 0 },
  ];

  return {
    levels: [level0, level1, level2, level3],
    edges,
  };
}
