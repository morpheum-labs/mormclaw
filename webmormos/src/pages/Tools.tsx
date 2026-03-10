import { useState, useEffect } from 'react';
import {
  Wrench,
  Search,
  ChevronDown,
  ChevronRight,
  Terminal,
  Package,
  ShieldCheck,
  ShieldX,
} from 'lucide-react';
import { parse } from 'smol-toml';
import type { ToolSpec, CliTool } from '@/types/api';
import { getTools, getCliTools, getConfig } from '@/lib/api';

function getToolPolicyFromConfig(configToml: string): {
  allowedTools: string[];
  deniedTools: string[];
  executionPolicy: string;
} {
  try {
    const obj = parse(configToml) as Record<string, unknown>;
    const agent = obj?.agent as Record<string, unknown> | undefined;
    const slots = (obj?.plugins as Record<string, unknown>)?.slots as Record<string, unknown> | undefined;
    const allowedTools = (agent?.allowed_tools as string[] | undefined) ?? [];
    const deniedTools = (agent?.denied_tools as string[] | undefined) ?? [];
    const executionPolicy = (slots?.executionPolicy as string | undefined) ?? '';
    return { allowedTools, deniedTools, executionPolicy };
  } catch {
    return { allowedTools: [], deniedTools: [], executionPolicy: '' };
  }
}

export default function Tools() {
  const [tools, setTools] = useState<ToolSpec[]>([]);
  const [cliTools, setCliTools] = useState<CliTool[]>([]);
  const [search, setSearch] = useState('');
  const [expandedTool, setExpandedTool] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toolPolicy, setToolPolicy] = useState<{
    allowedTools: string[];
    deniedTools: string[];
    executionPolicy: string;
  }>({ allowedTools: [], deniedTools: [], executionPolicy: '' });

  useEffect(() => {
    Promise.all([getTools(), getCliTools()])
      .then(([t, c]) => {
        setTools(t);
        setCliTools(c);
      })
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));

    getConfig()
      .then((config) => setToolPolicy(getToolPolicyFromConfig(config)))
      .catch(() => {});
  }, []);

  const filtered = tools.filter(
    (t) =>
      t.name.toLowerCase().includes(search.toLowerCase()) ||
      t.description.toLowerCase().includes(search.toLowerCase()),
  );

  const filteredCli = cliTools.filter(
    (t) =>
      t.name.toLowerCase().includes(search.toLowerCase()) ||
      t.category.toLowerCase().includes(search.toLowerCase()),
  );

  if (error) {
    return (
      <div className="p-6">
        <div className="rounded-lg bg-red-900/30 border border-red-700 p-4 text-red-300">
          Failed to load tools: {error}
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-2 border-blue-500 border-t-transparent" />
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      {/* Search */}
      <div className="relative max-w-md">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-500" />
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search tools..."
          className="w-full bg-gray-900 border border-gray-700 rounded-lg pl-10 pr-4 py-2.5 text-sm text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
      </div>

      {/* Agent Tools Grid */}
      <div>
        <div className="flex flex-wrap items-center gap-2 mb-4">
          <Wrench className="h-5 w-5 text-blue-400" />
          <h2 className="text-base font-semibold text-white">
            Agent Tools ({filtered.length})
          </h2>
          {toolPolicy.executionPolicy === 'mormos-allowlist' && toolPolicy.allowedTools.length > 0 && (
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-amber-900/50 text-amber-300 border border-amber-700/50">
              <ShieldCheck className="h-3 w-3" />
              Allowlist ({toolPolicy.allowedTools.length} tools)
            </span>
          )}
          {toolPolicy.executionPolicy === 'mormos-allowlist' && toolPolicy.deniedTools.length > 0 && (
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-red-900/30 text-red-300 border border-red-700/50">
              <ShieldX className="h-3 w-3" />
              Denylist ({toolPolicy.deniedTools.length} tools)
            </span>
          )}
        </div>

        {filtered.length === 0 ? (
          <p className="text-sm text-gray-500">No tools match your search.</p>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            {filtered.map((tool) => {
              const isExpanded = expandedTool === tool.name;
              return (
                <div
                  key={tool.name}
                  className="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden"
                >
                  <button
                    onClick={() =>
                      setExpandedTool(isExpanded ? null : tool.name)
                    }
                    className="w-full text-left p-4 hover:bg-gray-800/50 transition-colors"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex items-center gap-2 min-w-0">
                        <Package className="h-4 w-4 text-blue-400 flex-shrink-0 mt-0.5" />
                        <h3 className="text-sm font-semibold text-white truncate">
                          {tool.name}
                        </h3>
                        {toolPolicy.executionPolicy === 'mormos-allowlist' &&
                          toolPolicy.allowedTools.length > 0 &&
                          toolPolicy.allowedTools.includes(tool.name) && (
                            <span
                              className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-900/50 text-emerald-300 border border-emerald-700/50 flex-shrink-0"
                              title="In allowlist"
                            >
                              <ShieldCheck className="h-2.5 w-2.5" />
                              Allowed
                            </span>
                          )}
                      </div>
                      {isExpanded ? (
                        <ChevronDown className="h-4 w-4 text-gray-400 flex-shrink-0" />
                      ) : (
                        <ChevronRight className="h-4 w-4 text-gray-400 flex-shrink-0" />
                      )}
                    </div>
                    <p className="text-sm text-gray-400 mt-2 line-clamp-2">
                      {tool.description}
                    </p>
                  </button>

                  {isExpanded && tool.parameters && (
                    <div className="border-t border-gray-800 p-4">
                      <p className="text-xs text-gray-500 mb-2 font-medium uppercase tracking-wider">
                        Parameter Schema
                      </p>
                      <pre className="text-xs text-gray-300 bg-gray-950 rounded-lg p-3 overflow-x-auto max-h-64 overflow-y-auto">
                        {JSON.stringify(tool.parameters, null, 2)}
                      </pre>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* CLI Tools Section */}
      {filteredCli.length > 0 && (
        <div>
          <div className="flex items-center gap-2 mb-4">
            <Terminal className="h-5 w-5 text-green-400" />
            <h2 className="text-base font-semibold text-white">
              CLI Tools ({filteredCli.length})
            </h2>
          </div>

          <div className="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-800">
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">
                    Name
                  </th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">
                    Path
                  </th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">
                    Version
                  </th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">
                    Category
                  </th>
                </tr>
              </thead>
              <tbody>
                {filteredCli.map((tool) => (
                  <tr
                    key={tool.name}
                    className="border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors"
                  >
                    <td className="px-4 py-3 text-white font-medium">
                      {tool.name}
                    </td>
                    <td className="px-4 py-3 text-gray-400 font-mono text-xs truncate max-w-[200px]">
                      {tool.path}
                    </td>
                    <td className="px-4 py-3 text-gray-400">
                      {tool.version ?? '-'}
                    </td>
                    <td className="px-4 py-3">
                      <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-800 text-gray-300 capitalize">
                        {tool.category}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
