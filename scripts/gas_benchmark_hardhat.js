/*
  Hardhat gas benchmark simulation for Dongle contract workflows.

  Baseline notes (update after first run):
  - registerProject: ~310k gas
  - addReview: ~170k gas
  - updateReview: ~120k gas
  - requestVerification: ~210k gas
  - listProjects (view): ~0 gas (eth_call)

  Usage (Hardhat project context):
  1) npx hardhat run scripts/gas_benchmark_hardhat.js --network <network>
  2) Replace ABI/address placeholders below with deployed contract values.
*/

const { ethers } = require("hardhat");

async function measure(label, txPromise) {
  const tx = await txPromise;
  const receipt = await tx.wait();
  const gasUsed = receipt.gasUsed.toString();
  console.log(`${label}: ${gasUsed} gas`);
  return gasUsed;
}

async function main() {
  const [admin, reviewer] = await ethers.getSigners();

  // Replace with real deployed address and ABI if running against a live deployment.
  const contractAddress = process.env.DONGLE_CONTRACT_ADDRESS || "0x0000000000000000000000000000000000000000";
  const abi = [
    "function register_project(tuple(address owner,string name,string slug,string description,string category,string website,string license,string logo_cid,string metadata_cid,string[] tags,(string,string)[] social_links,uint64 launch_timestamp,string bounty_url)) returns (uint64)",
    "function add_review(uint64 project_id,address reviewer,uint32 rating,string comment_cid)",
    "function update_review(uint64 project_id,address reviewer,uint32 rating,string comment_cid)",
    "function request_verification(uint64 project_id,address requester,string evidence_cid)",
    "function list_projects(uint64 start_id,uint32 limit) view returns (tuple(uint64 id,address owner,string name,string slug,string description,string category)[])"
  ];

  const contract = new ethers.Contract(contractAddress, abi, admin);

  const now = Math.floor(Date.now() / 1000);
  const projectParams = {
    owner: admin.address,
    name: "Benchmark Project",
    slug: `benchmark-${now}`,
    description: "Gas benchmark registration",
    category: "infra",
    website: "",
    license: "MIT",
    logo_cid: "",
    metadata_cid: "",
    tags: [],
    social_links: [],
    launch_timestamp: now,
    bounty_url: ""
  };

  const registrationGas = await measure("register_project", contract.register_project(projectParams));

  // Use known project id in your environment. Replace with emitted ID parsing if ABI/events are available.
  const projectId = Number(process.env.BENCHMARK_PROJECT_ID || "1");

  const reviewerContract = contract.connect(reviewer);
  const addReviewGas = await measure(
    "add_review",
    reviewerContract.add_review(projectId, reviewer.address, 5, "")
  );

  const updateReviewGas = await measure(
    "update_review",
    reviewerContract.update_review(projectId, reviewer.address, 4, "")
  );

  const verificationGas = await measure(
    "request_verification",
    contract.request_verification(projectId, admin.address, "")
  );

  // View/listing estimate done off-chain as eth_call (no direct gas charged on chain).
  await contract.list_projects(0, 20);
  console.log("list_projects: view call executed (no on-chain gas charged)");

  console.log("\nSummary:");
  console.log(`register_project=${registrationGas}`);
  console.log(`add_review=${addReviewGas}`);
  console.log(`update_review=${updateReviewGas}`);
  console.log(`request_verification=${verificationGas}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
