# ArthaChain Terraform Variables
# Following Sui/Aptos/Sei industry patterns

variable "aws_region" {
  description = "AWS region for ArthaChain deployment"
  type        = string
  default     = "us-east-1"
}

variable "environment" {
  description = "Environment name (testnet, mainnet, dev)"
  type        = string
  default     = "testnet"
  
  validation {
    condition     = contains(["testnet", "mainnet", "dev"], var.environment)
    error_message = "Environment must be one of: testnet, mainnet, dev"
  }
}

variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "availability_zones" {
  description = "Availability zones for deployment"
  type        = list(string)
  default     = ["us-east-1a", "us-east-1b", "us-east-1c"]
}

variable "instance_type" {
  description = "EC2 instance type for ArthaChain nodes"
  type        = string
  default     = "t3.medium"
  
  validation {
    condition     = can(regex("^t3\\.", var.instance_type)) || can(regex("^m5\\.", var.instance_type)) || can(regex("^c5\\.", var.instance_type))
    error_message = "Instance type must be t3, m5, or c5 series for cost optimization"
  }
}

variable "ami_id" {
  description = "AMI ID for ArthaChain nodes (Ubuntu 22.04 LTS recommended)"
  type        = string
  default     = "ami-0c7217cdde317cfec"  # Ubuntu 22.04 LTS in us-east-1
  
  validation {
    condition     = can(regex("^ami-", var.ami_id))
    error_message = "AMI ID must start with 'ami-'"
  }
}

variable "desired_capacity" {
  description = "Desired number of ArthaChain nodes"
  type        = number
  default     = 5
  
  validation {
    condition     = var.desired_capacity >= 3 && var.desired_capacity <= 20
    error_message = "Desired capacity must be between 3 and 20 nodes"
  }
}

variable "min_size" {
  description = "Minimum number of ArthaChain nodes"
  type        = number
  default     = 3
  
  validation {
    condition     = var.min_size >= 3 && var.min_size <= var.desired_capacity
    error_message = "Min size must be at least 3 and not greater than desired capacity"
  }
}

variable "max_size" {
  description = "Maximum number of ArthaChain nodes"
  type        = number
  default     = 20
  
  validation {
    condition     = var.max_size >= var.desired_capacity
    error_message = "Max size must be greater than or equal to desired capacity"
  }
}

variable "ssh_allowed_cidrs" {
  description = "CIDR blocks allowed SSH access"
  type        = list(string)
  default     = ["0.0.0.0/0"]  # WARNING: Restrict this in production
  
  validation {
    condition     = length(var.ssh_allowed_cidrs) > 0
    error_message = "At least one SSH CIDR must be specified"
  }
}

variable "node_storage_size" {
  description = "Storage size in GB for ArthaChain nodes"
  type        = number
  default     = 100
  
  validation {
    condition     = var.node_storage_size >= 50 && var.node_storage_size <= 1000
    error_message = "Storage size must be between 50 and 1000 GB"
  }
}

variable "enable_monitoring" {
  description = "Enable CloudWatch monitoring"
  type        = bool
  default     = true
}

variable "enable_backup" {
  description = "Enable automated backups"
  type        = bool
  default     = true
}

variable "backup_retention_days" {
  description = "Number of days to retain backups"
  type        = number
  default     = 7
  
  validation {
    condition     = var.backup_retention_days >= 1 && var.backup_retention_days <= 30
    error_message = "Backup retention must be between 1 and 30 days"
  }
}

variable "enable_auto_scaling" {
  description = "Enable auto-scaling based on CPU utilization"
  type        = bool
  default     = true
}

variable "target_cpu_utilization" {
  description = "Target CPU utilization for auto-scaling"
  type        = number
  default     = 70
  
  validation {
    condition     = var.target_cpu_utilization >= 50 && var.target_cpu_utilization <= 90
    error_message = "Target CPU utilization must be between 50 and 90 percent"
  }
}

variable "enable_ssl" {
  description = "Enable SSL/TLS termination at load balancer"
  type        = bool
  default     = false  # Enable in production
}

variable "ssl_certificate_arn" {
  description = "ARN of SSL certificate for HTTPS"
  type        = string
  default     = ""
  
  validation {
    condition     = var.enable_ssl == false || (var.enable_ssl == true && var.ssl_certificate_arn != "")
    error_message = "SSL certificate ARN must be provided when SSL is enabled"
  }
}

variable "enable_waf" {
  description = "Enable AWS WAF for DDoS protection"
  type        = bool
  default     = false  # Enable in production
}

variable "enable_cloudfront" {
  description = "Enable CloudFront CDN for global distribution"
  type        = bool
  default     = false  # Enable in production
}

variable "tags" {
  description = "Additional tags for resources"
  type        = map(string)
  default     = {}
}
