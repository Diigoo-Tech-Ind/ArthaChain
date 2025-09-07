# ArthaChain Infrastructure as Code
# Following Sui/Aptos/Sei industry patterns

terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
  
  backend "s3" {
    bucket = "arthachain-terraform-state"
    key    = "testnet/terraform.tfstate"
    region = "us-east-1"
  }
}

# AWS Provider configuration
provider "aws" {
  region = var.aws_region
  
  default_tags {
    tags = {
      Project     = "ArthaChain"
      Environment = var.environment
      ManagedBy   = "Terraform"
      Network     = "testnet"
    }
  }
}

# VPC for ArthaChain network
resource "aws_vpc" "arthachain_vpc" {
  cidr_block           = var.vpc_cidr
  enable_dns_hostnames = true
  enable_dns_support   = true
  
  tags = {
    Name = "arthachain-vpc-${var.environment}"
  }
}

# Internet Gateway
resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.arthachain_vpc.id
  
  tags = {
    Name = "arthachain-igw-${var.environment}"
  }
}

# Public subnets across availability zones
resource "aws_subnet" "public" {
  count             = length(var.availability_zones)
  vpc_id            = aws_vpc.arthachain_vpc.id
  cidr_block        = cidrsubnet(var.vpc_cidr, 8, count.index)
  availability_zone = var.availability_zones[count.index]
  
  map_public_ip_on_launch = true
  
  tags = {
    Name = "arthachain-public-${var.availability_zones[count.index]}-${var.environment}"
  }
}

# Route table for public subnets
resource "aws_route_table" "public" {
  vpc_id = aws_vpc.arthachain_vpc.id
  
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.main.id
  }
  
  tags = {
    Name = "arthachain-public-rt-${var.environment}"
  }
}

# Route table associations
resource "aws_route_table_association" "public" {
  count          = length(var.availability_zones)
  subnet_id      = aws_subnet.public[count.index].id
  route_table_id = aws_route_table.public.id
}

# Security group for ArthaChain nodes
resource "aws_security_group" "arthachain_node" {
  name        = "arthachain-node-sg-${var.environment}"
  description = "Security group for ArthaChain nodes"
  vpc_id      = aws_vpc.arthachain_vpc.id
  
  # API port (like Sui:9000, Aptos:8080)
  ingress {
    from_port   = 8080
    to_port     = 8080
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "API/RPC access"
  }
  
  # P2P port (like Sui:8084, Aptos:6180)
  ingress {
    from_port   = 8084
    to_port     = 8084
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "P2P networking"
  }
  
  # Metrics port (like Sui:9184)
  ingress {
    from_port   = 9184
    to_port     = 9184
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "Metrics and monitoring"
  }
  
  # SSH access (for management)
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = var.ssh_allowed_cidrs
    description = "SSH access"
  }
  
  # All outbound traffic
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = {
    Name = "arthachain-node-sg-${var.environment}"
  }
}

# Security group for load balancer
resource "aws_security_group" "load_balancer" {
  name        = "arthachain-lb-sg-${var.environment}"
  description = "Security group for ArthaChain load balancer"
  vpc_id      = aws_vpc.arthachain_vpc.id
  
  # HTTP
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  # HTTPS
  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  # All outbound traffic
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = {
    Name = "arthachain-lb-sg-${var.environment}"
  }
}

# Application Load Balancer
resource "aws_lb" "arthachain" {
  name               = "arthachain-alb-${var.environment}"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.load_balancer.id]
  subnets            = aws_subnet.public[*].id
  
  enable_deletion_protection = false
  
  tags = {
    Name = "arthachain-alb-${var.environment}"
  }
}

# Target group for ArthaChain nodes
resource "aws_lb_target_group" "arthachain" {
  name     = "arthachain-tg-${var.environment}"
  port     = 8080
  protocol = "HTTP"
  vpc_id   = aws_vpc.arthachain_vpc.id
  
  health_check {
    enabled             = true
    healthy_threshold   = 2
    interval            = 30
    matcher             = "200"
    path                = "/health"
    port                = "traffic-port"
    protocol            = "HTTP"
    timeout             = 10
    unhealthy_threshold = 3
  }
  
  tags = {
    Name = "arthachain-tg-${var.environment}"
  }
}

# Load balancer listener
resource "aws_lb_listener" "arthachain" {
  load_balancer_arn = aws_lb.arthachain.arn
  port              = "80"
  protocol          = "HTTP"
  
  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.arthachain.arn
  }
}

# Launch template for ArthaChain nodes
resource "aws_launch_template" "arthachain_node" {
  name_prefix   = "arthachain-node-${var.environment}"
  image_id      = var.ami_id
  instance_type = var.instance_type
  
  network_interfaces {
    associate_public_ip_address = true
    security_groups             = [aws_security_group.arthachain_node.id]
  }
  
  user_data = base64encode(templatefile("${path.module}/user_data.sh", {
    environment = var.environment
    node_type  = "validator"
  }))
  
  iam_instance_profile {
    name = aws_iam_instance_profile.arthachain_node.name
  }
  
  tag_specifications {
    resource_type = "instance"
    tags = {
      Name = "arthachain-node-${var.environment}"
    }
  }
}

# Auto Scaling Group
resource "aws_autoscaling_group" "arthachain" {
  name                = "arthachain-asg-${var.environment}"
  desired_capacity    = var.desired_capacity
  max_size            = var.max_size
  min_size            = var.min_size
  target_group_arns   = [aws_lb_target_group.arthachain.arn]
  vpc_zone_identifier = aws_subnet.public[*].id
  
  launch_template {
    id      = aws_launch_template.arthachain_node.id
    version = "$Latest"
  }
  
  tag {
    key                 = "Name"
    value               = "arthachain-node-${var.environment}"
    propagate_at_launch = true
  }
}

# IAM role for ArthaChain nodes
resource "aws_iam_role" "arthachain_node" {
  name = "arthachain-node-role-${var.environment}"
  
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })
}

# IAM instance profile
resource "aws_iam_instance_profile" "arthachain_node" {
  name = "arthachain-node-profile-${var.environment}"
  role = aws_iam_role.arthachain_node.name
}

# IAM policy for ArthaChain nodes
resource "aws_iam_role_policy" "arthachain_node" {
  name = "arthachain-node-policy-${var.environment}"
  role = aws_iam_role.arthachain_node.id
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ec2:DescribeInstances",
          "ec2:DescribeTags",
          "ec2:DescribeRegions"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = "*"
      }
    ]
  })
}

# Route53 hosted zone for ArthaChain
resource "aws_route53_zone" "arthachain" {
  name = "arthachain.in"
  
  tags = {
    Environment = var.environment
  }
}

# DNS records for seed nodes
resource "aws_route53_record" "seed1" {
  zone_id = aws_route53_zone.arthachain.zone_id
  name    = "seed1.arthachain.in"
  type    = "A"
  ttl     = "300"
  
  # This will be updated with actual IP after deployment
  records = ["0.0.0.0"]
}

resource "aws_route53_record" "seed2" {
  zone_id = aws_route53_zone.arthachain.zone_id
  name    = "seed2.arthachain.in"
  type    = "A"
  ttl     = "300"
  
  records = ["0.0.0.0"]
}

resource "aws_route53_record" "seed3" {
  zone_id = aws_route53_zone.arthachain.zone_id
  name    = "seed3.arthachain.in"
  type    = "A"
  ttl     = "300"
  
  records = ["0.0.0.0"]
}

# API endpoint DNS record
resource "aws_route53_record" "api" {
  zone_id = aws_route53_zone.arthachain.zone_id
  name    = "api.arthachain.in"
  type    = "A"
  
  alias {
    name                   = aws_lb.arthachain.dns_name
    zone_id                = aws_lb.arthachain.zone_id
    evaluate_target_health = true
  }
}
