// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title JobRegistry v1 (ABI FROZEN)
 * @notice Registry for AI/compute jobs with DID integration
 * @dev Namespace: ArthaJobID
 */
contract JobRegistry {
    enum JobStatus { Queued, Running, Completed, Failed, Cancelled }
    
    struct Job {
        bytes32 jobId;
        bytes32 aiid;           // ArthaAIID
        bytes32 datasetId;      // ArthaDatasetID
        bytes32 paramsHash;     // Hash of job parameters
        bytes32 submitterDid;   // Artha-DID of submitter
        address submitter;
        bytes32 assignedNode;   // NodeCert pubkey
        uint64 submitTime;
        uint64 startTime;
        uint64 endTime;
        JobStatus status;
        bytes32 resultCid;      // SVDB CID for job output
        bytes32[] checkpointCids; // Checkpoints during execution
    }

    mapping(bytes32 => Job) public jobs;
    mapping(bytes32 => bytes32[]) public submitterJobs;  // submitterDid => jobIds
    mapping(bytes32 => bytes32[]) public nodeJobs;       // node => jobIds
    bytes32[] public jobList;
    
    uint256 public totalJobs;

    event JobCreated(
        bytes32 indexed jobId,
        bytes32 indexed aiid,
        bytes32 indexed submitterDid,
        bytes32 datasetId
    );
    event JobAssigned(bytes32 indexed jobId, bytes32 indexed assignedNode);
    event JobStarted(bytes32 indexed jobId, uint64 startTime);
    event JobCompleted(bytes32 indexed jobId, bytes32 resultCid, uint64 endTime);
    event JobFailed(bytes32 indexed jobId, uint64 endTime);
    event JobCancelled(bytes32 indexed jobId);
    event JobCheckpoint(bytes32 indexed jobId, bytes32 checkpointCid, uint256 checkpointIndex);

    error JobAlreadyExists(bytes32 jobId);
    error JobNotFound(bytes32 jobId);
    error InvalidJobStatus(bytes32 jobId, JobStatus current, JobStatus required);
    error NotJobSubmitter(bytes32 jobId, address caller);

    /**
     * @notice Create a new job
     * @param aiid AIID for the model
     * @param datasetId Dataset to use
     * @param paramsHash Hash of job parameters
     * @param submitterDid DID of submitter
     */
    function createJob(
        bytes32 aiid,
        bytes32 datasetId,
        bytes32 paramsHash,
        bytes32 submitterDid
    ) external returns (bytes32) {
        bytes32 jobId = keccak256(abi.encodePacked(
            aiid,
            datasetId,
            paramsHash,
            submitterDid,
            block.timestamp
        ));
        
        if (jobs[jobId].submitTime != 0) revert JobAlreadyExists(jobId);
        
        jobs[jobId] = Job({
            jobId: jobId,
            aiid: aiid,
            datasetId: datasetId,
            paramsHash: paramsHash,
            submitterDid: submitterDid,
            submitter: msg.sender,
            assignedNode: bytes32(0),
            submitTime: uint64(block.timestamp),
            startTime: 0,
            endTime: 0,
            status: JobStatus.Queued,
            resultCid: bytes32(0),
            checkpointCids: new bytes32[](0)
        });
        
        submitterJobs[submitterDid].push(jobId);
        jobList.push(jobId);
        totalJobs++;
        
        emit JobCreated(jobId, aiid, submitterDid, datasetId);
        
        return jobId;
    }

    /**
     * @notice Assign job to a node
     * @param jobId Job ID
     * @param assignedNode Node public key
     */
    function assignJob(bytes32 jobId, bytes32 assignedNode) external {
        Job storage job = jobs[jobId];
        
        if (job.submitTime == 0) revert JobNotFound(jobId);
        if (job.status != JobStatus.Queued) {
            revert InvalidJobStatus(jobId, job.status, JobStatus.Queued);
        }
        
        job.assignedNode = assignedNode;
        nodeJobs[assignedNode].push(jobId);
        
        emit JobAssigned(jobId, assignedNode);
    }

    /**
     * @notice Start job execution
     * @param jobId Job ID
     */
    function startJob(bytes32 jobId) external {
        Job storage job = jobs[jobId];
        
        if (job.submitTime == 0) revert JobNotFound(jobId);
        if (job.status != JobStatus.Queued) {
            revert InvalidJobStatus(jobId, job.status, JobStatus.Queued);
        }
        
        job.status = JobStatus.Running;
        job.startTime = uint64(block.timestamp);
        
        emit JobStarted(jobId, job.startTime);
    }

    /**
     * @notice Add checkpoint during job execution
     * @param jobId Job ID
     * @param checkpointCid CID of checkpoint data
     */
    function addCheckpoint(bytes32 jobId, bytes32 checkpointCid) external {
        Job storage job = jobs[jobId];
        
        if (job.submitTime == 0) revert JobNotFound(jobId);
        if (job.status != JobStatus.Running) {
            revert InvalidJobStatus(jobId, job.status, JobStatus.Running);
        }
        
        job.checkpointCids.push(checkpointCid);
        
        emit JobCheckpoint(jobId, checkpointCid, job.checkpointCids.length - 1);
    }

    /**
     * @notice Complete a job
     * @param jobId Job ID
     * @param resultCid CID of job result
     */
    function completeJob(bytes32 jobId, bytes32 resultCid) external {
        Job storage job = jobs[jobId];
        
        if (job.submitTime == 0) revert JobNotFound(jobId);
        if (job.status != JobStatus.Running) {
            revert InvalidJobStatus(jobId, job.status, JobStatus.Running);
        }
        
        job.status = JobStatus.Completed;
        job.endTime = uint64(block.timestamp);
        job.resultCid = resultCid;
        
        emit JobCompleted(jobId, resultCid, job.endTime);
    }

    /**
     * @notice Mark job as failed
     * @param jobId Job ID
     */
    function failJob(bytes32 jobId) external {
        Job storage job = jobs[jobId];
        
        if (job.submitTime == 0) revert JobNotFound(jobId);
        if (job.status != JobStatus.Running) {
            revert InvalidJobStatus(jobId, job.status, JobStatus.Running);
        }
        
        job.status = JobStatus.Failed;
        job.endTime = uint64(block.timestamp);
        
        emit JobFailed(jobId, job.endTime);
    }

    /**
     * @notice Cancel a job
     * @param jobId Job ID
     */
    function cancelJob(bytes32 jobId) external {
        Job storage job = jobs[jobId];
        
        if (job.submitTime == 0) revert JobNotFound(jobId);
        if (job.submitter != msg.sender) revert NotJobSubmitter(jobId, msg.sender);
        if (job.status != JobStatus.Queued && job.status != JobStatus.Running) {
            revert InvalidJobStatus(jobId, job.status, JobStatus.Queued);
        }
        
        job.status = JobStatus.Cancelled;
        job.endTime = uint64(block.timestamp);
        
        emit JobCancelled(jobId);
    }

    /**
     * @notice Get job details
     * @param jobId Job ID
     * @return Job struct
     */
    function getJob(bytes32 jobId) external view returns (Job memory) {
        if (jobs[jobId].submitTime == 0) revert JobNotFound(jobId);
        return jobs[jobId];
    }

    /**
     * @notice Get jobs by submitter
     * @param submitterDid Submitter DID
     * @return Array of job IDs
     */
    function getJobsBySubmitter(bytes32 submitterDid) external view returns (bytes32[] memory) {
        return submitterJobs[submitterDid];
    }

    /**
     * @notice Get jobs by node
     * @param nodeId Node public key
     * @return Array of job IDs
     */
    function getJobsByNode(bytes32 nodeId) external view returns (bytes32[] memory) {
        return nodeJobs[nodeId];
    }

    /**
     * @notice Get job checkpoints
     * @param jobId Job ID
     * @return Array of checkpoint CIDs
     */
    function getCheckpoints(bytes32 jobId) external view returns (bytes32[] memory) {
        if (jobs[jobId].submitTime == 0) revert JobNotFound(jobId);
        return jobs[jobId].checkpointCids;
    }

    /**
     * @notice Get job statistics
     * @return queued, running, completed, failed
     */
    function getJobStats() external view returns (
        uint256 queued,
        uint256 running,
        uint256 completed,
        uint256 failed
    ) {
        for (uint256 i = 0; i < jobList.length; i++) {
            Job storage job = jobs[jobList[i]];
            if (job.status == JobStatus.Queued) queued++;
            else if (job.status == JobStatus.Running) running++;
            else if (job.status == JobStatus.Completed) completed++;
            else if (job.status == JobStatus.Failed) failed++;
        }
    }
}

