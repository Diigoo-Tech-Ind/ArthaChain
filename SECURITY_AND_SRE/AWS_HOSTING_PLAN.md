## AWS Hosting Plan (using credits)

Scope: Host ArthaChain public APIs on AWS.

Architecture (MVP):
- Compute: ECS Fargate (or EC2 Auto Scaling if needed for bare-metal perf)
- Networking: ALB + HTTPS (ACM), private subnets for tasks, public for ALB
- Images: ECR for `blockchain_node` Docker image
- Config/Secrets: SSM Parameter Store + Secrets Manager
- Observability: CloudWatch Logs + Metrics; optional OTLP to AWS Distro for OpenTelemetry
- Data: EBS volumes for node data (if stateful), or S3 for logs/archives
- Security: IAM roles for tasks, Security Groups per tier, WAF on ALB

Rollout Steps:
1) Build & push image: `docker build` → ECR → tag by commit
2) Provision VPC + ALB + ECS cluster (Terraform under `infrastructure/terraform/`)
3) Create ECS task definition for API service (CPU/RAM, env from SSM)
4) Service with desired count ≥2 across AZs; target group healthcheck `/api/v1/test/health`
5) ACM cert + Route53: `api.arthachain.online`
6) Logs/traces: ship to CloudWatch; enable OTel exporter (optional)
7) Auto Scaling: CPU/Latency based; WAF rate-limits for basic DoS protection

Credentials & Access:
- Use AWS Credits account; least-privileged IAM for CI to push to ECR and update service

Cost Controls:
- Start with t4g.* EC2 or small Fargate tasks; scale by demand

Cutover Plan:
- Blue/green deployment via ECS deployment controller; rollback on failed health


