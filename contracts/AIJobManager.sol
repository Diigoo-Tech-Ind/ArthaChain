// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title AIJobManager
/// @notice Manages AI training, inference, agent, and federated learning jobs
contract AIJobManager {
    enum JobType { Train, Infer, Agent, Federated, Evolution }
    enum JobStatus { Queued, Assigned, Running, Completed, Failed, Cancelled }

    struct Job {
        bytes32 jobId;
        JobType jobType;
        JobStatus status;
        address submitter;
        bytes32 submitterDid;
        bytes32 modelId;
        bytes32 datasetId;
        bytes32 paramsHash;
        bytes32 assignedNode;
        uint256 budget;
        uint256 spent;
        uint64 submittedAt;
        uint64 startedAt;
        uint64 completedAt;
        bytes32 outputCid;
        bytes32[] artifacts;
    }

    struct TrainParams {
        uint256 epochs;
        uint256 batchSize;
        string optimizer;
        bytes32 checkpointInterval;
    }

    struct InferParams {
        bytes32 inputCid;
        string mode; // "batch", "realtime", "stream"
        uint256 maxTokens;
    }

    mapping(bytes32 => Job) public jobs;
    mapping(bytes32 => bytes32[]) public jobProofs;
    mapping(address => bytes32[]) public submitterJobs;
    mapping(bytes32 => bytes32[]) public nodeJobs; // nodePubkey -> jobIds

    uint256 public totalJobs;
    uint256 public activeJobs;

    event JobSubmitted(
        bytes32 indexed jobId,
        JobType jobType,
        address indexed submitter,
        bytes32 modelId,
        uint256 budget
    );
    event JobAssigned(bytes32 indexed jobId, bytes32 indexed nodePubkey);
    event JobStatusUpdated(bytes32 indexed jobId, JobStatus newStatus);
    event JobCompleted(bytes32 indexed jobId, bytes32 outputCid, uint256 finalCost);
    event JobFailed(bytes32 indexed jobId, string reason);

    function submitTrain(
        bytes32 modelId,
        bytes32 datasetId,
        bytes32 paramsHash,
        uint256 epochs,
        uint256 budget,
        bytes32 submitterDid
    ) external payable returns (bytes32) {
        require(msg.value >= budget, "Insufficient payment");

        bytes32 jobId = keccak256(abi.encodePacked(
            msg.sender,
            modelId,
            datasetId,
            block.timestamp,
            totalJobs
        ));

        jobs[jobId] = Job({
            jobId: jobId,
            jobType: JobType.Train,
            status: JobStatus.Queued,
            submitter: msg.sender,
            submitterDid: submitterDid,
            modelId: modelId,
            datasetId: datasetId,
            paramsHash: paramsHash,
            assignedNode: bytes32(0),
            budget: budget,
            spent: 0,
            submittedAt: uint64(block.timestamp),
            startedAt: 0,
            completedAt: 0,
            outputCid: bytes32(0),
            artifacts: new bytes32[](0)
        });

        submitterJobs[msg.sender].push(jobId);
        totalJobs++;
        activeJobs++;

        emit JobSubmitted(jobId, JobType.Train, msg.sender, modelId, budget);
        return jobId;
    }

    function submitInfer(
        bytes32 modelId,
        bytes32 inputCid,
        string calldata mode,
        uint256 budget,
        bytes32 submitterDid
    ) external payable returns (bytes32) {
        require(msg.value >= budget, "Insufficient payment");

        bytes32 jobId = keccak256(abi.encodePacked(
            msg.sender,
            modelId,
            inputCid,
            block.timestamp,
            totalJobs
        ));

        jobs[jobId] = Job({
            jobId: jobId,
            jobType: JobType.Infer,
            status: JobStatus.Queued,
            submitter: msg.sender,
            submitterDid: submitterDid,
            modelId: modelId,
            datasetId: inputCid, // Reuse field for input
            paramsHash: keccak256(abi.encodePacked(mode)),
            assignedNode: bytes32(0),
            budget: budget,
            spent: 0,
            submittedAt: uint64(block.timestamp),
            startedAt: 0,
            completedAt: 0,
            outputCid: bytes32(0),
            artifacts: new bytes32[](0)
        });

        submitterJobs[msg.sender].push(jobId);
        totalJobs++;
        activeJobs++;

        emit JobSubmitted(jobId, JobType.Infer, msg.sender, modelId, budget);
        return jobId;
    }

    function submitAgent(
        bytes32 agentSpecCid,
        uint256 budget,
        bytes32 submitterDid
    ) external payable returns (bytes32) {
        require(msg.value >= budget, "Insufficient payment");

        bytes32 jobId = keccak256(abi.encodePacked(
            msg.sender,
            agentSpecCid,
            block.timestamp,
            totalJobs
        ));

        jobs[jobId] = Job({
            jobId: jobId,
            jobType: JobType.Agent,
            status: JobStatus.Queued,
            submitter: msg.sender,
            submitterDid: submitterDid,
            modelId: agentSpecCid, // Reuse field
            datasetId: bytes32(0),
            paramsHash: bytes32(0),
            assignedNode: bytes32(0),
            budget: budget,
            spent: 0,
            submittedAt: uint64(block.timestamp),
            startedAt: 0,
            completedAt: 0,
            outputCid: bytes32(0),
            artifacts: new bytes32[](0)
        });

        submitterJobs[msg.sender].push(jobId);
        totalJobs++;
        activeJobs++;

        emit JobSubmitted(jobId, JobType.Agent, msg.sender, agentSpecCid, budget);
        return jobId;
    }

    function assignJob(bytes32 jobId, bytes32 nodePubkey) external {
        // Called by scheduler
        Job storage job = jobs[jobId];
        require(job.status == JobStatus.Queued, "Job not queued");

        job.assignedNode = nodePubkey;
        job.status = JobStatus.Assigned;

        nodeJobs[nodePubkey].push(jobId);

        emit JobAssigned(jobId, nodePubkey);
    }

    function updateStatus(bytes32 jobId, JobStatus newStatus) external {
        Job storage job = jobs[jobId];
        require(job.jobId != bytes32(0), "Job not found");

        job.status = newStatus;

        if (newStatus == JobStatus.Running && job.startedAt == 0) {
            job.startedAt = uint64(block.timestamp);
        }

        emit JobStatusUpdated(jobId, newStatus);
    }

    function completeJob(
        bytes32 jobId,
        bytes32 outputCid,
        uint256 computeCost,
        bytes32[] calldata artifacts
    ) external {
        Job storage job = jobs[jobId];
        require(job.status == JobStatus.Running, "Job not running");
        require(computeCost <= job.budget, "Cost exceeds budget");

        job.status = JobStatus.Completed;
        job.completedAt = uint64(block.timestamp);
        job.outputCid = outputCid;
        job.spent = computeCost;

        for (uint i = 0; i < artifacts.length; i++) {
            job.artifacts.push(artifacts[i]);
        }

        activeJobs--;

        // Transfer payment to compute provider
        // In production: verify proof-of-compute first
        // payable(nodeAddress).transfer(computeCost);

        // Refund excess
        uint256 refund = job.budget - computeCost;
        if (refund > 0) {
            payable(job.submitter).transfer(refund);
        }

        emit JobCompleted(jobId, outputCid, computeCost);
    }

    function failJob(bytes32 jobId, string calldata reason) external {
        Job storage job = jobs[jobId];
        require(job.status == JobStatus.Running || job.status == JobStatus.Assigned, "Invalid state");

        job.status = JobStatus.Failed;
        job.completedAt = uint64(block.timestamp);

        activeJobs--;

        // Refund
        payable(job.submitter).transfer(job.budget - job.spent);

        emit JobFailed(jobId, reason);
    }

    function getJob(bytes32 jobId) external view returns (Job memory) {
        return jobs[jobId];
    }

    function getSubmitterJobs(address submitter) external view returns (bytes32[] memory) {
        return submitterJobs[submitter];
    }

    function getNodeJobs(bytes32 nodePubkey) external view returns (bytes32[] memory) {
        return nodeJobs[nodePubkey];
    }

    function getJobArtifacts(bytes32 jobId) external view returns (bytes32[] memory) {
        return jobs[jobId].artifacts;
    }

    function addProof(bytes32 jobId, bytes32 proofId) external {
        jobProofs[jobId].push(proofId);
    }

    function getJobProofs(bytes32 jobId) external view returns (bytes32[] memory) {
        return jobProofs[jobId];
    }
}

