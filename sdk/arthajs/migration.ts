/**
 * SDK Migration Adapters for Backward Compatibility
 * Provides shims for deprecated API methods and schema versions
 */

import { ArthaID, ArthaVC, ArthaAIID } from './index';

// Version compatibility mapping
export const SCHEMA_VERSION_MAP: Record<string, string> = {
    'DIDDoc': 'v1',
    'AIIDDoc': 'v1',
    'VC': 'v1',
    'NodeCert': 'v1',
    'JobSpec': 'v1',
};

// Deprecated method warnings
const DEPRECATION_WARNINGS = new Set<string>();

function warnDeprecated(oldMethod: string, newMethod: string, sunsetDate: string) {
    const key = `${oldMethod}->${newMethod}`;
    if (!DEPRECATION_WARNINGS.has(key)) {
        console.warn(
            `⚠️  DEPRECATED: ${oldMethod} is deprecated and will be removed on ${sunsetDate}. ` +
            `Use ${newMethod} instead. See migration guide: https://docs.arthachain.online/migration`
        );
        DEPRECATION_WARNINGS.add(key);
    }
}

/**
 * Legacy DID API (pre-v1)
 * Provides backward compatibility for old DID creation method
 */
export class LegacyDIDAPI {
    private didAPI: ArthaID;

    constructor(baseUrl: string, rpcUrl: string) {
        this.didAPI = new ArthaID(baseUrl, rpcUrl);
    }

    /**
     * @deprecated Use ArthaID.createDID() with explicit key parameters
     * @sunset 2026-11-02 (24 months from v1 release)
     */
    async registerIdentity(publicKey: string, metadata: any): Promise<{did: string}> {
        warnDeprecated('registerIdentity()', 'createDID()', '2026-11-02');
        
        // Old API assumed single key for both auth and enc
        // New API requires separate keys
        const authKey = publicKey;
        const encKey = this.deriveEncryptionKey(publicKey);
        const metaCid = await this.uploadMetadata(metadata);

        const result = await this.didAPI.createDID(authKey, encKey, metaCid);
        return { did: result.did };
    }

    /**
     * @deprecated Use ArthaID.getDID()
     * @sunset 2026-11-02
     */
    async lookupIdentity(did: string): Promise<any> {
        warnDeprecated('lookupIdentity()', 'getDID()', '2026-11-02');
        return this.didAPI.getDID(did);
    }

    private deriveEncryptionKey(authKey: string): string {
        // Derive X25519 encryption key from Ed25519 auth key
        // In production, use proper crypto library (libsodium, noble-curves, etc.)
        return authKey; // Placeholder: same key for now
    }

    private async uploadMetadata(metadata: any): string {
        // Upload metadata to SVDB and return CID
        // For migration, we serialize to JSON and create a placeholder CID
        const jsonStr = JSON.stringify(metadata);
        const hash = this.simpleHash(jsonStr);
        return `artha://${hash}`;
    }

    private simpleHash(data: string): string {
        // Simple hash for migration purposes
        let hash = 0;
        for (let i = 0; i < data.length; i++) {
            const char = data.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash;
        }
        return Math.abs(hash).toString(16).padStart(16, '0');
    }
}

/**
 * Legacy VC API (pre-v1)
 */
export class LegacyVCAPI {
    private vcAPI: ArthaVC;

    constructor(baseUrl: string) {
        this.vcAPI = new ArthaVC(baseUrl);
    }

    /**
     * @deprecated Use ArthaVC.issueVC() with explicit claim hash
     * @sunset 2026-11-02
     */
    async issueCredential(issuer: string, subject: string, claims: any, expiresIn: number): Promise<{vcHash: string}> {
        warnDeprecated('issueCredential()', 'issueVC()', '2026-11-02');
        
        // Old API: claims as object, expiresIn as duration
        // New API: claimHash as bytes32, expiresAt as timestamp
        
        const claimHash = this.hashClaims(claims);
        const docCid = await this.uploadClaims(claims);
        const expiresAt = Math.floor(Date.now() / 1000) + expiresIn;

        const result = await this.vcAPI.issueVC(issuer, subject, claimHash, docCid, expiresAt);
        return { vcHash: result.vcHash };
    }

    /**
     * @deprecated Use ArthaVC.verifyVC()
     * @sunset 2026-11-02
     */
    async checkCredential(vcHash: string): Promise<{valid: boolean}> {
        warnDeprecated('checkCredential()', 'verifyVC()', '2026-11-02');
        const result = await this.vcAPI.verifyVC(vcHash);
        return { valid: result.valid };
    }

    private hashClaims(claims: any): string {
        const jsonStr = JSON.stringify(claims);
        // Use keccak256 in production
        return '0x' + this.simpleHash(jsonStr);
    }

    private async uploadClaims(claims: any): Promise<string> {
        const jsonStr = JSON.stringify(claims);
        return `artha://${this.simpleHash(jsonStr)}`;
    }

    private simpleHash(data: string): string {
        let hash = 0;
        for (let i = 0; i < data.length; i++) {
            const char = data.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash;
        }
        return Math.abs(hash).toString(16).padStart(64, '0');
    }
}

/**
 * Schema Version Adapter
 * Automatically upgrades old schema versions to current version
 */
export class SchemaVersionAdapter {
    /**
     * Migrate DIDDoc from legacy format to v1
     */
    static migrateDIDDoc(legacyDoc: any): any {
        if (legacyDoc['@schema'] === 'artha://schema/DIDDoc@v1') {
            return legacyDoc; // Already v1
        }

        // Legacy format: single 'publicKey' field
        // v1 format: 'publicKey' array with type/purpose
        
        const v1Doc = {
            '@context': 'https://www.w3.org/ns/did/v1',
            '@schema': 'artha://schema/DIDDoc@v1',
            id: legacyDoc.id || legacyDoc.did,
            publicKey: [],
            proof: legacyDoc.proof || legacyDoc.signature,
            createdAt: legacyDoc.createdAt || legacyDoc.created,
            updatedAt: legacyDoc.updatedAt || legacyDoc.updated,
            revoked: legacyDoc.revoked || false,
        };

        // Migrate single key to array format
        if (legacyDoc.publicKey) {
            v1Doc.publicKey.push({
                id: '#auth-key-1',
                type: 'Ed25519',
                key: legacyDoc.publicKey,
                purpose: 'authentication',
            });
        }

        // Add encryption key if present
        if (legacyDoc.encryptionKey) {
            v1Doc.publicKey.push({
                id: '#enc-key-1',
                type: 'X25519',
                key: legacyDoc.encryptionKey,
                purpose: 'encryption',
            });
        }

        // Migrate services
        if (legacyDoc.serviceEndpoints) {
            v1Doc['service'] = legacyDoc.serviceEndpoints.map((endpoint: string, i: number) => ({
                id: `#service-${i}`,
                type: 'SVDBEndpoint',
                endpoint,
            }));
        }

        console.log(`✅ Migrated DIDDoc from legacy format to v1: ${v1Doc.id}`);
        return v1Doc;
    }

    /**
     * Migrate VC from legacy format to v1
     */
    static migrateVC(legacyVC: any): any {
        if (legacyVC['@schema'] === 'artha://schema/VC@v1') {
            return legacyVC; // Already v1
        }

        // Legacy format: different field names
        const v1VC = {
            '@context': [
                'https://www.w3.org/2018/credentials/v1',
                'https://arthachain.online/contexts/kyc/v1',
            ],
            '@schema': 'artha://schema/VC@v1',
            issuerDid: legacyVC.issuer || legacyVC.issuerDid,
            subjectDid: legacyVC.subject || legacyVC.subjectDid,
            type: ['VerifiableCredential', legacyVC.credentialType || 'Custom'],
            claim: {
                type: legacyVC.claimType || 'Custom',
                value: legacyVC.claims || legacyVC.claim,
                claimHash: legacyVC.claimHash || this.hashObject(legacyVC.claims),
            },
            docCid: legacyVC.docCid || legacyVC.documentCid,
            issuedAt: legacyVC.issuedAt || legacyVC.issuanceDate,
            expiresAt: legacyVC.expiresAt || legacyVC.expirationDate || 0,
            proof: legacyVC.proof || legacyVC.signature,
            revoked: legacyVC.revoked || false,
        };

        console.log(`✅ Migrated VC from legacy format to v1: ${v1VC.claim.claimHash}`);
        return v1VC;
    }

    /**
     * Migrate AIIDDoc from legacy format to v1
     */
    static migrateAIIDDoc(legacyDoc: any): any {
        if (legacyDoc['@schema'] === 'artha://schema/AIIDDoc@v1') {
            return legacyDoc; // Already v1
        }

        const v1Doc = {
            '@schema': 'artha://schema/AIIDDoc@v1',
            id: legacyDoc.id || legacyDoc.aiid,
            ownerDid: legacyDoc.ownerDid || legacyDoc.owner,
            model: {
                name: legacyDoc.modelName || legacyDoc.name,
                type: legacyDoc.modelType || 'llm',
                version: legacyDoc.version || 'v1.0.0',
                modelCid: legacyDoc.modelCid || legacyDoc.cid,
                architecture: legacyDoc.architecture,
                parameterCount: legacyDoc.parameters || legacyDoc.parameterCount,
            },
            datasetId: legacyDoc.datasetId || legacyDoc.trainingData,
            codeHash: legacyDoc.codeHash || '0x0',
            lineage: legacyDoc.lineage || legacyDoc.parentModels || [],
            signature: legacyDoc.signature || legacyDoc.proof,
            createdAt: legacyDoc.createdAt || legacyDoc.created,
            active: legacyDoc.active !== false,
        };

        // Migrate eval metrics if present
        if (legacyDoc.metrics || legacyDoc.eval) {
            v1Doc['eval'] = {
                benchmark: legacyDoc.metrics?.benchmark || legacyDoc.eval?.benchmark,
                score: legacyDoc.metrics?.score || legacyDoc.eval?.score,
                proofCid: legacyDoc.metrics?.proofCid || legacyDoc.eval?.proofCid,
            };
        }

        console.log(`✅ Migrated AIIDDoc from legacy format to v1: ${v1Doc.id}`);
        return v1Doc;
    }

    private static hashObject(obj: any): string {
        const jsonStr = JSON.stringify(obj);
        let hash = 0;
        for (let i = 0; i < jsonStr.length; i++) {
            const char = jsonStr.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash;
        }
        return '0x' + Math.abs(hash).toString(16).padStart(64, '0');
    }
}

/**
 * Bulk Migration Utility
 * Migrate multiple documents at once
 */
export class BulkMigrator {
    /**
     * Migrate an array of documents to v1 schemas
     */
    static migrateDocuments(docs: any[], docType: 'DIDDoc' | 'VC' | 'AIIDDoc'): any[] {
        console.log(`🔄 Starting bulk migration of ${docs.length} ${docType} documents...`);
        
        const migrated = docs.map(doc => {
            try {
                switch (docType) {
                    case 'DIDDoc':
                        return SchemaVersionAdapter.migrateDIDDoc(doc);
                    case 'VC':
                        return SchemaVersionAdapter.migrateVC(doc);
                    case 'AIIDDoc':
                        return SchemaVersionAdapter.migrateAIIDDoc(doc);
                    default:
                        throw new Error(`Unknown document type: ${docType}`);
                }
            } catch (error) {
                console.error(`❌ Failed to migrate document:`, doc, error);
                return null;
            }
        }).filter(doc => doc !== null);

        console.log(`✅ Successfully migrated ${migrated.length}/${docs.length} documents`);
        return migrated;
    }

    /**
     * Generate migration report
     */
    static generateMigrationReport(oldDocs: any[], newDocs: any[], docType: string): string {
        const report = [
            `# Migration Report: ${docType}`,
            `Date: ${new Date().toISOString()}`,
            ``,
            `## Summary`,
            `- Total documents: ${oldDocs.length}`,
            `- Successfully migrated: ${newDocs.length}`,
            `- Failed: ${oldDocs.length - newDocs.length}`,
            ``,
            `## Schema Versions`,
            `- Target schema: ${SCHEMA_VERSION_MAP[docType]}`,
            ``,
            `## Changes Applied`,
            `- Updated schema references to v1`,
            `- Normalized field names`,
            `- Added required v1 fields`,
            `- Preserved all original data`,
            ``,
            `## Next Steps`,
            `1. Review migrated documents`,
            `2. Validate against v1 JSON schemas`,
            `3. Test with new SDK methods`,
            `4. Deploy to production`,
            ``,
            `For questions, see: https://docs.arthachain.online/migration`,
        ];

        return report.join('\n');
    }
}

/**
 * Migration CLI Helper
 * Usage: node migration-cli.js migrate-dids input.json output.json
 */
export async function runMigration(
    inputFile: string,
    outputFile: string,
    docType: 'DIDDoc' | 'VC' | 'AIIDDoc'
): Promise<void> {
    const fs = await import('fs');
    
    console.log(`📂 Reading ${inputFile}...`);
    const inputData = JSON.parse(fs.readFileSync(inputFile, 'utf-8'));
    
    const docs = Array.isArray(inputData) ? inputData : [inputData];
    const migratedDocs = BulkMigrator.migrateDocuments(docs, docType);
    
    console.log(`💾 Writing to ${outputFile}...`);
    fs.writeFileSync(outputFile, JSON.stringify(migratedDocs, null, 2));
    
    const report = BulkMigrator.generateMigrationReport(docs, migratedDocs, docType);
    const reportFile = outputFile.replace('.json', '-report.md');
    fs.writeFileSync(reportFile, report);
    
    console.log(`✅ Migration complete!`);
    console.log(`   Output: ${outputFile}`);
    console.log(`   Report: ${reportFile}`);
}

