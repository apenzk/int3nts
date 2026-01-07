import { VerifierTest } from "@/components/verifier/VerifierTest";
import Link from "next/link";

export default function DebugPage() {
  return (
    <div className="min-h-screen p-8">
      <main className="max-w-4xl mx-auto">
        <div className="mb-6">
          <Link 
            href="/" 
            className="text-blue-400 hover:text-blue-300 text-sm mb-4 inline-block"
          >
            ← Back to Home
          </Link>
          <h1 className="text-4xl font-bold mb-2">Debug</h1>
          <p className="text-lg text-gray-400 mb-8">Verifier API Testing</p>
        </div>
        
        <VerifierTest />
      </main>
    </div>
  );
}

