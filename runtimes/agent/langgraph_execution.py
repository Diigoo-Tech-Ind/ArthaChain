#!/usr/bin/env python3
"""
LangGraph Execution - Real Implementation
State machine-based agent execution with LangGraph
"""

import os
import sys
import json
from typing import Dict, Any, List
from langgraph.graph import StateGraph, END
from langgraph.prebuilt import ToolExecutor, ToolInvocation
from langchain_openai import ChatOpenAI
from langchain_core.messages import HumanMessage, AIMessage, ToolMessage

# State definition
class AgentState:
    def __init__(self):
        self.messages = []
        self.goal = ""
        self.tools_used = []
        self.iterations = 0

def create_langgraph_agent(tools: List, llm):
    """Create LangGraph state machine agent"""
    
    def should_continue(state: AgentState) -> str:
        """Determine next step"""
        last_message = state.messages[-1]
        
        # Check if agent wants to use tools
        if hasattr(last_message, 'tool_calls') and last_message.tool_calls:
            return "continue"
        
        # Check if goal is achieved
        if state.iterations >= 10:
            return "end"
            
        return "continue"
    
    def call_tools(state: AgentState) -> AgentState:
        """Execute tool calls"""
        last_message = state.messages[-1]
        
        if hasattr(last_message, 'tool_calls'):
            tool_executor = ToolExecutor(tools)
            tool_invocations = [
                ToolInvocation(tool=tc["name"], tool_input=tc["args"])
                for tc in last_message.tool_calls
            ]
            
            results = tool_executor.batch(tool_invocations)
            tool_messages = [
                ToolMessage(content=str(result), tool_call_id=tc["id"])
                for result, tc in zip(results, last_message.tool_calls)
            ]
            
            state.messages.extend(tool_messages)
            state.tools_used.extend([tc["name"] for tc in last_message.tool_calls])
        
        return state
    
    def call_model(state: AgentState) -> AgentState:
        """Call LLM"""
        messages = state.messages
        response = llm.invoke(messages)
        state.messages.append(response)
        state.iterations += 1
        return state
    
    # Build graph
    workflow = StateGraph(AgentState)
    workflow.add_node("agent", call_model)
    workflow.add_node("tools", call_tools)
    
    workflow.set_entry_point("agent")
    workflow.add_conditional_edges(
        "agent",
        should_continue,
        {
            "continue": "tools",
            "end": END
        }
    )
    workflow.add_edge("tools", "agent")
    
    return workflow.compile()

def run_langgraph_agent(goal: str, tools: List) -> Dict[str, Any]:
    """Run LangGraph agent"""
    llm = ChatOpenAI(model="gpt-4", temperature=0.7)
    agent = create_langgraph_agent(tools, llm)
    
    initial_state = AgentState()
    initial_state.goal = goal
    initial_state.messages = [HumanMessage(content=goal)]
    
    result = agent.invoke(initial_state)
    
    return {
        "result": result.messages[-1].content if result.messages else "",
        "tools_used": result.tools_used,
        "iterations": result.iterations,
    }

if __name__ == "__main__":
    goal = os.environ.get("AGENT_GOAL", "Complete the task")
    tools_str = os.environ.get("AGENT_TOOLS", "search,storage")
    
    from tools import search_tool, storage_tool
    tools = []
    if "search" in tools_str:
        tools.append(search_tool)
    if "storage" in tools_str:
        tools.append(storage_tool)
    
    result = run_langgraph_agent(goal, tools)
    print(json.dumps(result, indent=2))

