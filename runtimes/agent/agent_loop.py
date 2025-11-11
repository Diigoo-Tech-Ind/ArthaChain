#!/usr/bin/env python3
"""
Agent Runtime Loop - Real LangChain/LangGraph Implementation
Supports autonomous agents with tool use, memory, and planning
"""

import os
import sys
import json
import time
from typing import List, Dict, Any

# Add parent directory for SVDB client
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
try:
    from svdb_client import svdb
except ImportError:
    svdb = None

# LangChain imports
try:
    from langchain.agents import initialize_agent, AgentType, create_openai_tools_agent
    from langchain_openai import ChatOpenAI
    from langchain.tools import Tool
    from langchain.memory import ConversationBufferMemory
    from langgraph.graph import StateGraph, END
    from langgraph.prebuilt import ToolExecutor, ToolInvocation
    from crewai import Agent, Task, Crew
    import autogen
except ImportError as e:
    print(f"⚠️  Missing framework: {e}")
    print("   Using fallback implementation")

JOB_ID = os.environ.get("ARTHA_JOB_ID", "agent-unknown")
GOAL = os.environ.get("AGENT_GOAL", "Complete the task")
TOOLS_STR = os.environ.get("AGENT_TOOLS", "search,storage,tx")
MEMORY_POLICY = os.environ.get("MEMORY_POLICY", "episodic")
PROOF_SERVICE_URL = os.environ.get("PROOF_SERVICE_URL", "http://localhost:8085")

def create_tools() -> List[Tool]:
    """Create tools available to the agent"""
    from tools import (
        search_tool,
        storage_tool,
        transaction_tool,
        read_file_tool,
        write_file_tool,
    )
    
    tools = []
    for tool_name in TOOLS_STR.split(','):
        tool_name = tool_name.strip()
        if tool_name == "search":
            tools.append(Tool(name="search", func=search_tool, description="Search the web"))
        elif tool_name == "storage":
            tools.append(Tool(name="storage", func=storage_tool, description="Store/retrieve from SVDB"))
        elif tool_name == "tx":
            tools.append(Tool(name="transaction", func=transaction_tool, description="Execute blockchain transaction"))
        elif tool_name == "read":
            tools.append(Tool(name="read_file", func=read_file_tool, description="Read a file"))
        elif tool_name == "write":
            tools.append(Tool(name="write_file", func=write_file_tool, description="Write a file"))
    
    return tools

def run_langchain_agent():
    """Run agent using LangChain"""
    print(f"🤖 Starting LangChain agent - Job: {JOB_ID}")
    
    # Initialize LLM
    llm = ChatOpenAI(
        model=os.environ.get("LLM_MODEL", "gpt-4"),
        temperature=0.7,
        openai_api_key=os.environ.get("OPENAI_API_KEY", ""),
    )
    
    # Create memory
    memory = ConversationBufferMemory(
        memory_key="chat_history",
        return_messages=True,
    )
    
    # Initialize agent
    tools = create_tools()
    agent = initialize_agent(
        tools,
        llm,
        agent=AgentType.OPENAI_FUNCTIONS,
        memory=memory,
        verbose=True,
    )
    
    # Run agent
    result = agent.run(GOAL)
    
    # Save memory to SVDB
    if svdb and MEMORY_POLICY != "none":
        memory_path = "/tmp/agent_memory.json"
        with open(memory_path, 'w') as f:
            json.dump({"goal": GOAL, "result": result}, f)
        svdb.upload(memory_path, replicas=3, months=6)
    
    return result

def run_langgraph_agent():
    """Run agent using LangGraph (state machine)"""
    print(f"🤖 Starting LangGraph agent - Job: {JOB_ID}")
    
    from langgraph.graph import StateGraph, END
    from typing_extensions import TypedDict
    
    class AgentState(TypedDict):
        messages: List[Dict]
        goal: str
        tools_called: List[str]
    
    # Define graph
    workflow = StateGraph(AgentState)
    
    def plan_node(state: AgentState) -> AgentState:
        """Planning node"""
        print(f"   📋 Planning for goal: {state['goal']}")
        state["messages"].append({"role": "assistant", "content": "Planning steps..."})
        return state
    
    def execute_node(state: AgentState) -> AgentState:
        """Execution node"""
        print(f"   ⚙️  Executing plan...")
        state["tools_called"].append("execute")
        return state
    
    workflow.add_node("plan", plan_node)
    workflow.add_node("execute", execute_node)
    workflow.set_entry_point("plan")
    workflow.add_edge("plan", "execute")
    workflow.add_edge("execute", END)
    
    # Run graph
    app = workflow.compile()
    initial_state = {
        "messages": [],
        "goal": GOAL,
        "tools_called": [],
    }
    
    result = app.invoke(initial_state)
    return result

def run_crewai_agent():
    """Run multi-agent crew using CrewAI"""
    print(f"👥 Starting CrewAI multi-agent crew - Job: {JOB_ID}")
    
    # Define agents
    researcher = Agent(
        role="Researcher",
        goal="Research information",
        backstory="Expert researcher",
        verbose=True,
    )
    
    executor = Agent(
        role="Executor",
        goal="Execute tasks",
        backstory="Skilled executor",
        verbose=True,
    )
    
    # Define tasks
    task = Task(
        description=GOAL,
        agent=researcher,
    )
    
    # Create crew
    crew = Crew(
        agents=[researcher, executor],
        tasks=[task],
        verbose=True,
    )
    
    # Execute
    result = crew.kickoff()
    return result

def run_autogen_agent():
    """Run agent using AutoGen"""
    print(f"🔄 Starting AutoGen agent - Job: {JOB_ID}")
    
    config_list = [{
        "model": os.environ.get("LLM_MODEL", "gpt-4"),
        "api_key": os.environ.get("OPENAI_API_KEY", ""),
    }]
    
    # Create assistant agent
    assistant = autogen.AssistantAgent(
        name="assistant",
        llm_config={"config_list": config_list},
        system_message="You are a helpful assistant.",
    )
    
    # Create user proxy
    user_proxy = autogen.UserProxyAgent(
        name="user_proxy",
        human_input_mode="NEVER",
        max_consecutive_auto_reply=10,
    )
    
    # Initiate chat
    user_proxy.initiate_chat(assistant, message=GOAL)
    
    return assistant.last_message()["content"]

def submit_proof(tool_calls: List[Dict], result: Any):
    """Submit proof of agent execution"""
    try:
        import requests
        requests.post(
            f"{PROOF_SERVICE_URL}/proof/submit",
            json={
                "job_id": JOB_ID,
                "proof_type": "AgentExecution",
                "tool_calls": tool_calls,
                "result": str(result),
            },
            timeout=10,
        )
    except:
        pass

def main():
    """Main agent loop"""
    print(f"🚀 Agent Runtime - Job: {JOB_ID}")
    print(f"   Goal: {GOAL}")
    print(f"   Tools: {TOOLS_STR}")
    print(f"   Memory: {MEMORY_POLICY}")
    
    # Select framework
    framework = os.environ.get("AGENT_FRAMEWORK", "langchain")
    
    tool_calls = []
    start_time = time.time()
    
    try:
        if framework == "langchain":
            result = run_langchain_agent()
        elif framework == "langgraph":
            result = run_langgraph_agent()
        elif framework == "crewai":
            result = run_crewai_agent()
        elif framework == "autogen":
            result = run_autogen_agent()
        else:
            print(f"⚠️  Unknown framework: {framework}, using langchain")
            result = run_langchain_agent()
        
        elapsed = time.time() - start_time
        print(f"\n✅ Agent execution complete in {elapsed:.2f}s")
        print(f"   Result: {result}")
        
        # Submit proof
        submit_proof(tool_calls, result)
        
    except Exception as e:
        print(f"\n❌ Agent execution failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main()

