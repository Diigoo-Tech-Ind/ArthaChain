# ArthaAIN v1 - Agent Runtime
# Supports LangChain, LangGraph, CrewAI for autonomous agents

FROM python:3.11-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install agent frameworks
RUN pip install --no-cache-dir \
    langchain==0.1.0 \
    langgraph==0.0.20 \
    langchain-community==0.0.13 \
    langchain-openai==0.0.2 \
    openai==1.6.1 \
    crewai==0.11.0 \
    autogen-agentchat==0.2.0

# Install tool libraries
RUN pip install --no-cache-dir \
    requests==2.31.0 \
    beautifulsoup4==4.12.2 \
    duckduckgo-search==4.1.0 \
    wikipedia==1.4.0 \
    python-dotenv==1.0.0

# Install Artha SVDB client
RUN pip install --no-cache-dir blake3

# Create working directories
RUN mkdir -p /agent /memory /tools /app

# Copy agent scripts
COPY agent_loop.py /app/agent_loop.py
COPY tools.py /app/tools.py
COPY memory.py /app/memory.py

# Set environment variables
ENV PYTHONUNBUFFERED=1

# Default command
CMD ["python3", "/app/agent_loop.py"]

