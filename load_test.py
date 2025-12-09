#!/usr/bin/env python3
"""
ArthaChain Load Testing Script
Performs stress testing with simulated transaction traffic
"""

import asyncio
import aiohttp
import time
import json
from datetime import datetime
import statistics

# Configuration
API_URL = "http://localhost:8080"
CONCURRENT_REQUESTS = 100
TOTAL_REQUESTS = 1000
TIMEOUT = 30

class LoadTester:
    def __init__(self):
        self.results = []
        self.errors = []
        self.start_time = None
        self.end_time = None
    
    async def send_transaction(self, session, tx_id):
        """Send a single test transaction"""
        start = time.time()
        try:
            # Test transaction creation
            payload = {
                "from": f"0x{'1' * 40}",
                "to": f"0x{'2' * 40}",
                "value": "1000000000000000000",  # 1 ARTH
                "data": f"0xtest{tx_id:08x}",
                "nonce": tx_id
            }
            
            async with session.post(
                f"{API_URL}/api/transaction/send",
                json=payload,
                timeout=aiohttp.ClientTimeout(total=TIMEOUT)
            ) as response:
                latency = time.time() - start
                status = response.status
                
                if status == 200:
                    self.results.append({
                        'tx_id': tx_id,
                        'latency': latency,
                        'status': status,
                        'success': True
                    })
                else:
                    self.errors.append({
                        'tx_id': tx_id,
                        'latency': latency,
                        'status': status,
                        'error': f"HTTP {status}"
                    })
                    
        except asyncio.TimeoutError:
            self.errors.append({
                'tx_id': tx_id,
                'error': 'Timeout',
                'latency': TIMEOUT
            })
        except Exception as e:
            self.errors.append({
                'tx_id': tx_id,
                'error': str(e),
                'latency': time.time() - start
            })
    
    async def run_load_test(self):
        """Execute load test with concurrent requests"""
        print(f"🚀 Starting load test...")
        print(f"   Target: {API_URL}")
        print(f"   Concurrent: {CONCURRENT_REQUESTS}")
        print(f"   Total Requests: {TOTAL_REQUESTS}")
        print()
        
        self.start_time = time.time()
        
        async with aiohttp.ClientSession() as session:
            # Create batches of concurrent requests
            for batch_start in range(0, TOTAL_REQUESTS, CONCURRENT_REQUESTS):
                batch_end = min(batch_start + CONCURRENT_REQUESTS, TOTAL_REQUESTS)
                tasks = [
                    self.send_transaction(session, tx_id)
                    for tx_id in range(batch_start, batch_end)
                ]
                
                await asyncio.gather(*tasks)
                
                # Progress update
                completed = len(self.results) + len(self.errors)
                print(f"Progress: {completed}/{TOTAL_REQUESTS} "
                      f"({completed/TOTAL_REQUESTS*100:.1f}%)", end='\r')
        
        self.end_time = time.time()
        print()  # New line after progress
    
    def generate_report(self):
        """Generate detailed load testing report"""
        total_time = self.end_time - self.start_time
        total_requests = len(self.results) + len(self.errors)
        successful = len(self.results)
        failed = len(self.errors)
        
        # Calculate statistics
        if self.results:
            latencies = [r['latency'] for r in self.results]
            avg_latency = statistics.mean(latencies)
            min_latency = min(latencies)
            max_latency = max(latencies)
            p50_latency = statistics.median(latencies)
            p95_latency = statistics.quantiles(latencies, n=20)[18] if len(latencies) >= 20 else max_latency
            p99_latency = statistics.quantiles(latencies, n=100)[98] if len(latencies) >= 100 else max_latency
        else:
            avg_latency = min_latency = max_latency = p50_latency = p95_latency = p99_latency = 0
        
        tps = successful / total_time if total_time > 0 else 0
        
        report = f"""
╔═══════════════════════════════════════════════════════════════╗
║          ArthaChain Load Testing Report                       ║
╠═══════════════════════════════════════════════════════════════╣
║ Test Configuration:                                           ║
║   • Target URL: {API_URL:<46}║
║   • Concurrent Requests: {CONCURRENT_REQUESTS:<35}║
║   • Total Requests: {TOTAL_REQUESTS:<40}║
║   • Timeout: {TIMEOUT}s{' ' * 46}║
╠═══════════════════════════════════════════════════════════════╣
║ Overall Results:                                              ║
║   • Total Requests: {total_requests:<40}║
║   • Successful: {successful:<44}║
║   • Failed: {failed:<48}║
║   • Success Rate: {successful/total_requests*100 if total_requests > 0 else 0:.2f}%{' ' * 37}║
║   • Total Duration: {total_time:.2f}s{' ' * 36}║
║   • Throughput (TPS): {tps:.2f}{' ' * 35}║
╠═══════════════════════════════════════════════════════════════╣
║ Latency Statistics (successful requests only):                ║
║   • Average: {avg_latency*1000:.2f}ms{' ' * 38}║
║   • Minimum: {min_latency*1000:.2f}ms{' ' * 38}║
║   • Maximum: {max_latency*1000:.2f}ms{' ' * 38}║
║   • P50 (Median): {p50_latency*1000:.2f}ms{' ' * 33}║
║   • P95: {p95_latency*1000:.2f}ms{' ' * 43}║
║   • P99: {p99_latency*1000:.2f}ms{' ' * 43}║
╠═══════════════════════════════════════════════════════════════╣
║ Assessment:                                                   ║
"""
        
        # Add assessment
        if tps >= 100000:
            report += "║   ✅ EXCELLENT: Meets 100K+ TPS target{' ' * 23}║\n"
        elif tps >= 10000:
            report += "║   ✅ GOOD: Strong performance (10K+ TPS){' ' * 21}║\n"
        elif tps >= 1000:
            report += f"║   ⚠️  MODERATE: {tps:.0f} TPS (target: 100K+){' ' * 23}║\n"
        else:
            report += f"║   ❌ LOW: {tps:.0f} TPS (needs optimization){' ' * 25}║\n"
        
        if p99_latency < 0.2:  # 200ms target
            report += "║   ✅ Latency within target (<200ms P99){' ' * 21}║\n"
        else:
            report += f"║   ⚠️  Latency above target (P99: {p99_latency*1000:.0f}ms){' ' * 21}║\n"
        
        if successful / total_requests >= 0.999:
            report += "║   ✅ Availability target met (99.9%+){' ' * 24}║\n"
        else:
            report += f"║   ⚠️  Availability below target ({successful/total_requests*100:.2f}%){' ' * 18}║\n"
        
        report += "╚═══════════════════════════════════════════════════════════════╝\n"
        
        return report

async def main():
    """Main entry point"""
    print("=" * 65)
    print("  ArthaChain Load Testing Tool")
    print("  Testing blockchain transaction throughput and latency")
    print("=" * 65)
    print()
    
    # Check if API is accessible
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{API_URL}/health", timeout=aiohttp.ClientTimeout(total=5)) as response:
                if response.status == 200:
                    print(f"✅ API endpoint accessible at {API_URL}")
                else:
                    print(f"⚠️  API returned status {response.status}")
    except Exception as e:
        print(f"❌ Cannot connect to API at {API_URL}")
        print(f"   Error: {e}")
        print(f"\n💡 Make sure the blockchain node is running:")
        print(f"   docker-compose -f deploy/docker-compose.yml up -d")
        return
    
    print()
    
    # Run load test
    tester = LoadTester()
    await tester.run_load_test()
    
    # Generate and print report
    report = tester.generate_report()
    print(report)
    
    # Save report to file
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    report_file = f"load_test_report_{timestamp}.txt"
    
    with open(report_file, 'w') as f:
        f.write(report)
        f.write("\n\nDetailed Results:\n")
        f.write(json.dumps({
            'timestamp': timestamp,
            'config': {
                'api_url': API_URL,
                'concurrent_requests': CONCURRENT_REQUESTS,
                'total_requests': TOTAL_REQUESTS,
                'timeout': TIMEOUT
            },
            'successful_transactions': len(tester.results),
            'failed_transactions': len(tester.errors),
            'errors': tester.errors[:10]  # First 10 errors
        }, indent=2))
    
    print(f"\n📄 Detailed report saved to: {report_file}")

if __name__ == "__main__":
    asyncio.run(main())
