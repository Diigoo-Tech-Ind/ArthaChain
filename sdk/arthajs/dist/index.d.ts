import { ethers } from 'ethers';
/**
 * Transaction signer helper for interacting with Ethereum-compatible chains
 */
export declare class TransactionSigner {
    private wallet;
    private provider;
    constructor(privateKey: string, rpcUrl: string);
    signAndSend(tx: ethers.TransactionRequest): Promise<ethers.TransactionResponse>;
    call(tx: ethers.TransactionRequest): Promise<string>;
    getAddress(): string;
    getNonce(): Promise<number>;
    estimateGas(tx: ethers.TransactionRequest): Promise<bigint>;
}
export declare class ArthaJS {
    private baseUrl;
    constructor(baseUrl: string);
    quote(provider: string, cidUri: string): Promise<any>;
    /**
     * Settle a retrieval payment on-chain using local signing
     */
    settle(params: {
        signer: TransactionSigner;
        dealMarket: string;
        manifestRoot: string;
        bytesServed: number;
        provider: string;
        totalWei: bigint;
        gasPrice?: bigint;
        gasLimit?: bigint;
    }): Promise<ethers.TransactionResponse>;
    /**
     * Aggregate settlement with merkle root
     */
    settleAggregate(params: {
        signer: TransactionSigner;
        dealMarket: string;
        manifestRoot: string;
        merkleRoot: string;
        provider: string;
        totalWei: bigint;
        gasPrice?: bigint;
        gasLimit?: bigint;
    }): Promise<ethers.TransactionResponse>;
    /**
     * Aggregate settlement with per-leaf proof
     */
    settleAggregateWithProof(params: {
        signer: TransactionSigner;
        dealMarket: string;
        manifestRoot: string;
        merkleRoot: string;
        leaf: string;
        branch: string[];
        index: number;
        provider: string;
        totalWei: bigint;
        gasPrice?: bigint;
        gasLimit?: bigint;
    }): Promise<ethers.TransactionResponse>;
    uploadFile(filePath: string): Promise<string>;
    uploadFileWithEnvelope(filePath: string, envelope: {
        alg: string;
        salt_b64?: string;
        nonce_b64?: string;
        aad_b64?: string;
    }): Promise<string>;
    downloadToFile(cidUri: string, outPath: string, range?: {
        start?: number;
        end?: number;
    }): Promise<{
        status: number;
        bytes: number;
    }>;
    info(cidUri: string): Promise<any>;
    createDeal(params: {
        cid: string;
        size: number;
        replicas: number;
        months: number;
        maxPrice: number;
        signer?: TransactionSigner;
        dealMarket?: string;
        gasPrice?: bigint;
        gasLimit?: bigint;
    }): Promise<any>;
    setAccessPolicy(params: {
        cidUri: string;
        private: boolean;
        allowedDids?: string[];
        token?: string;
    }): Promise<any>;
    allowlistAdd(cidUri: string, did: string): Promise<any>;
    allowlistRemove(cidUri: string, did: string): Promise<any>;
    buildMerkleBranch(cid: string, index: number): Promise<{
        root: string;
        leaf: string;
        branch: string[];
        index: number;
    }>;
    /**
     * Submit proof payout using local signing
     */
    submitPayout(params: {
        signer: TransactionSigner;
        dealMarket: string;
        root: string;
        leaf: string;
        index: number;
        branch: string[];
    }): Promise<ethers.TransactionResponse>;
    getActiveProviders(rpcUrl: string, contract: string): Promise<any>;
    getProviderOffer(provider: string, rpcUrl: string, contract: string): Promise<any>;
    getProviderReputation(provider: string, rpcUrl: string, contract: string): Promise<any>;
    /**
     * Report latency using local signing
     */
    reportLatency(params: {
        signer: TransactionSigner;
        contract: string;
        provider: string;
        root: string;
        latencyMs: number;
    }): Promise<ethers.TransactionResponse>;
    porepProveSeal(params: {
        root: string;
        randomness: string;
        provider: string;
    }): Promise<any>;
    /**
     * Issue PoRep challenge using local signing
     */
    porepChallenge(params: {
        signer: TransactionSigner;
        contract: string;
        commitment: string;
    }): Promise<ethers.TransactionResponse>;
    aiTrain(params: {
        modelCid: string;
        datasetCid: string;
        epochs?: number;
        region?: string;
        zkEnabled?: boolean;
        gpuRequired?: boolean;
    }): Promise<any>;
    aiJobStatus(jobId: string): Promise<any>;
    aiDeploy(params: {
        modelCid: string;
        name?: string;
        region?: string;
        replicas?: number;
    }): Promise<any>;
    aiDeploymentStatus(deploymentId: string): Promise<any>;
    explorerProofs(cid: string): Promise<any>;
    estimateCost(params: {
        size: number;
        replicas: number;
        months: number;
        rpcUrl?: string;
        priceOracle?: string;
    }): Promise<any>;
}
export declare class ArthaID {
    private baseUrl;
    private signer?;
    constructor(baseUrl: string, signer?: TransactionSigner);
    /**
     * Create DID on-chain using local signing
     */
    createDID(authKey: string, encKey: string, metaCid: string, contract: string): Promise<{
        did: string;
        txHash: string;
    }>;
    getDID(did: string): Promise<any>;
    /**
     * Rotate DID keys using local signing
     */
    rotateKeys(contract: string, newAuthKey: string, newEncKey: string): Promise<{
        txHash: string;
    }>;
    /**
     * Revoke DID using local signing
     */
    revokeDID(contract: string): Promise<{
        txHash: string;
    }>;
    verifySignature(did: string, messageHash: string, signature: string): Promise<{
        valid: boolean;
    }>;
}
export declare class ArthaVC {
    private baseUrl;
    private signer?;
    constructor(baseUrl: string, signer?: TransactionSigner);
    /**
     * Issue VC on-chain using local signing
     */
    issueVC(contract: string, subjectDid: string, claimHash: string, docCid: string, expiresAt: number): Promise<{
        vcHash: string;
        txHash: string;
    }>;
    /**
     * Revoke VC using local signing
     */
    revokeVC(contract: string, vcHash: string): Promise<{
        txHash: string;
    }>;
    verifyVC(vcHash: string): Promise<{
        valid: boolean;
        vc: any;
    }>;
    getVCsBySubject(subjectDid: string): Promise<{
        vcs: any[];
    }>;
    hasClaimType(subjectDid: string, claimType: string): Promise<{
        has: boolean;
    }>;
}
export declare class ArthaAIID {
    private baseUrl;
    private signer?;
    constructor(baseUrl: string, signer?: TransactionSigner);
    /**
     * Create AI ID on-chain using local signing
     */
    createAIID(contract: string, modelCid: string, datasetId: string, codeHash: string, version: string): Promise<{
        aiid: string;
        txHash: string;
    }>;
    getAIID(aiid: string): Promise<any>;
    /**
     * Rotate AI ID using local signing
     */
    rotateAIID(contract: string, aiid: string, newModelCid: string, newVersion: string): Promise<{
        newAiid: string;
        txHash: string;
    }>;
    linkOwner(contract: string, aiid: string, ownerDid: string): Promise<{
        txHash: string;
    }>;
    getLineage(aiid: string): Promise<{
        lineage: string[];
    }>;
}
export declare class ArthaPolicy {
    private baseUrl;
    constructor(baseUrl: string);
    checkAccess(cid: string, did: string, sessionToken: string): Promise<{
        allowed: boolean;
        reason?: string;
    }>;
    createSession(did: string, scope: string[]): Promise<{
        sessionId: string;
        token: string;
    }>;
    revokeSession(sessionId: string): Promise<{
        success: boolean;
    }>;
}
export declare class ArthaAI {
    private baseUrl;
    constructor(baseUrl: string);
    scoreVCRisk(vcInput: any): Promise<{
        risk: number;
        reasonCodes: string[];
        threshold: boolean;
    }>;
    detectAnomaly(nodeMetrics: any): Promise<{
        anomalyScore: number;
        suggestedAction: string;
        anomalies: string[];
    }>;
    scoreReputation(reputationInput: any): Promise<{
        arthaScore: number;
        flags: string[];
        riskLevel: string;
    }>;
    verifyAuthenticity(receipt: any): Promise<{
        isAuthentic: boolean;
        confidence: number;
        provenance: string[];
    }>;
}
export declare class ArthaDataset {
    private baseUrl;
    private signer?;
    constructor(baseUrl: string, signer?: TransactionSigner | undefined);
    /**
     * Register dataset on-chain using local signing
     */
    register(contract: string, rootCid: string, licenseCid: string, tags: string[]): Promise<string>;
    list(owner?: string): Promise<any[]>;
    getInfo(datasetId: string): Promise<any>;
}
export declare class ArthaModel {
    private baseUrl;
    private signer?;
    constructor(baseUrl: string, signer?: TransactionSigner | undefined);
    /**
     * Register model on-chain using local signing
     */
    register(contract: string, params: {
        modelCid: string;
        architecture: string;
        baseModelId?: string;
        datasetId: string;
        codeHash: string;
        version: string;
        licenseCid?: string;
    }): Promise<string>;
    list(owner?: string): Promise<any[]>;
    getLineage(modelId: string): Promise<string[]>;
    addCheckpoint(modelId: string, checkpointCid: string, metricsJsonCid: string, step: number): Promise<void>;
    publish(modelId: string, checkpointCid: string): Promise<void>;
}
export declare class ArthaJob {
    private baseUrl;
    constructor(baseUrl: string);
    submitTrain(params: {
        modelId: string;
        datasetId: string;
        submitterDid: string;
        epochs: number;
        batchSize: number;
        learningRate: number;
        optimizer: string;
        budget: number;
    }): Promise<{
        jobId: string;
        status: string;
        estimatedCost: number;
        estimatedDurationSecs: number;
    }>;
    submitInfer(params: {
        modelId: string;
        inputCid?: string;
        inlineInput?: string;
        submitterDid: string;
        mode: string;
        maxTokens?: number;
        budget: number;
    }): Promise<{
        jobId: string;
        status: string;
    }>;
    submitAgent(params: {
        agentSpecCid: string;
        submitterDid: string;
        goal: string;
        tools: string[];
        memoryPolicy: string;
        budget: number;
    }): Promise<{
        jobId: string;
        status: string;
    }>;
    getStatus(jobId: string): Promise<any>;
    getLogs(jobId: string): Promise<string[]>;
    cancel(jobId: string): Promise<void>;
    getArtifacts(jobId: string): Promise<string[]>;
    getToolCalls(jobId: string): Promise<any[]>;
    recordToolCall(jobId: string, toolName: string, params: any, result: any): Promise<any>;
    getAgentMemory(jobId: string): Promise<any>;
    updateAgentMemory(jobId: string, memoryCid: string): Promise<void>;
}
export declare class ArthaFederated {
    private baseUrl;
    constructor(baseUrl: string);
    startRound(params: {
        modelId: string;
        datasetIds: string[];
        rounds: number;
        dp: boolean;
        budget: number;
    }): Promise<{
        fedId: string;
        status: string;
    }>;
    getRoundStatus(fedId: string): Promise<any>;
    submitGradient(fedId: string, weights: number[], sampleCount: number): Promise<any>;
    triggerAggregation(fedId: string): Promise<any>;
}
export declare class ArthaEvolution {
    private baseUrl;
    constructor(baseUrl: string);
    start(params: {
        searchSpaceCid: string;
        population: number;
        generations: number;
        budget: number;
    }): Promise<{
        evoId: string;
        status: string;
    }>;
    getStatus(evoId: string): Promise<any>;
    getPopulation(evoId: string): Promise<any>;
}
export declare class ArthaDeployment {
    private baseUrl;
    constructor(baseUrl: string);
    deploy(params: {
        modelId: string;
        endpoint: string;
        replicas: number;
        maxTokens: number;
    }): Promise<{
        deploymentId: string;
        endpointUrl: string;
    }>;
    getStatus(deploymentId: string): Promise<any>;
    scale(deploymentId: string, replicas: number): Promise<void>;
    undeploy(deploymentId: string): Promise<void>;
}
//# sourceMappingURL=index.d.ts.map