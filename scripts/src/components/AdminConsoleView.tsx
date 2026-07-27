import React, { useState } from 'react';
import { ShieldCheck, Cpu, Coins, AlertTriangle, UserCheck, ShieldAlert, CheckCircle2, XCircle, Plus, Key } from 'lucide-react';
import { UserAccount, VerificationRequest, DuplicateDispute, FeeConfig, AdminActionLog } from '../types';
import { contractEngine } from '../services/contractEngine';

interface AdminConsoleViewProps {
  currentUser: UserAccount;
  onUpdate: () => void;
}

export const AdminConsoleView: React.FC<AdminConsoleViewProps> = ({ currentUser, onUpdate }) => {
  const isAdmin = currentUser.role === 'admin';

  const [verifications, setVerifications] = useState<VerificationRequest[]>(contractEngine.getVerifications());
  const [disputes, setDisputes] = useState<DuplicateDispute[]>(contractEngine.getDisputes());
  const [feeConfig, setFeeConfig] = useState<FeeConfig>(contractEngine.getFeeConfig());
  const [adminLogs, setAdminLogs] = useState<AdminActionLog[]>(contractEngine.getAdminLogs());
  const [admins, setAdmins] = useState<string[]>(contractEngine.getAdmins());

  const [newAdminAddress, setNewAdminAddress] = useState('');
  const [regFee, setRegFee] = useState(feeConfig.registrationFee);
  const [verFee, setVerFee] = useState(feeConfig.verificationFee);
  const [feeToken, setFeeToken] = useState(feeConfig.feeToken);
  const [feeEnabled, setFeeEnabled] = useState(feeConfig.isFeeEnabled);

  const [actionMessage, setActionMessage] = useState<string | null>(null);

  const projects = contractEngine.getProjects();

  const handleApproveVerification = (id: number) => {
    try {
      contractEngine.approveVerification(currentUser.address, id, 365);
      setVerifications(contractEngine.getVerifications());
      setActionMessage(`Approved verification request #${id}`);
      onUpdate();
    } catch (e: any) {
      setActionMessage(`Error: ${e.message}`);
    }
  };

  const handleRejectVerification = (id: number) => {
    try {
      contractEngine.rejectVerification(currentUser.address, id, 'Requirements not met.');
      setVerifications(contractEngine.getVerifications());
      setActionMessage(`Rejected verification request #${id}`);
      onUpdate();
    } catch (e: any) {
      setActionMessage(`Error: ${e.message}`);
    }
  };

  const handleResolveDispute = (id: number, action: 'archive_project' | 'dismiss') => {
    try {
      contractEngine.resolveDuplicateDispute(currentUser.address, id, action, 'Admin moderation decision');
      setDisputes(contractEngine.getDisputes());
      setActionMessage(`Resolved dispute #${id} with action: ${action}`);
      onUpdate();
    } catch (e: any) {
      setActionMessage(`Error: ${e.message}`);
    }
  };

  const handleUpdateFees = (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const updated = contractEngine.updateFeeConfig(currentUser.address, {
        registrationFee: Number(regFee),
        verificationFee: Number(verFee),
        feeToken,
        isFeeEnabled: feeEnabled,
      });
      setFeeConfig(updated);
      setActionMessage('Fee Configuration & SAC Token settings updated successfully.');
      onUpdate();
    } catch (e: any) {
      setActionMessage(`Error: ${e.message}`);
    }
  };

  const handleAddAdmin = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newAdminAddress.trim()) return;
    try {
      contractEngine.addAdmin(currentUser.address, newAdminAddress.trim());
      setAdmins(contractEngine.getAdmins());
      setNewAdminAddress('');
      setActionMessage(`Added ${newAdminAddress.slice(0, 10)}... as contract Admin.`);
      onUpdate();
    } catch (e: any) {
      setActionMessage(`Error: ${e.message}`);
    }
  };

  if (!isAdmin) {
    return (
      <div className="text-center py-16 bg-slate-900/60 rounded-3xl border border-slate-800 space-y-3">
        <ShieldAlert className="w-12 h-12 text-amber-400 mx-auto" />
        <h2 className="text-xl font-bold text-white">Admin Privileges Required</h2>
        <p className="text-xs text-slate-400 max-w-md mx-auto">
          Your current account persona (<strong className="text-slate-200">{currentUser.name}</strong>) is a standard {currentUser.role}.
          Use the account switcher in the top right navbar to switch to <strong className="text-purple-400">Alice (Admin)</strong>.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Action Notification */}
      {actionMessage && (
        <div className="p-4 bg-purple-950/30 border border-purple-500/30 rounded-2xl text-xs text-purple-300 flex items-center justify-between">
          <span>{actionMessage}</span>
          <button onClick={() => setActionMessage(null)} className="text-purple-400 hover:text-white underline">Dismiss</button>
        </div>
      )}

      {/* Grid of Admin Controls */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Verification Queue */}
        <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 space-y-4">
          <div className="flex items-center justify-between border-b border-slate-800 pb-3">
            <h3 className="font-bold text-white text-base flex items-center space-x-2">
              <ShieldCheck className="w-5 h-5 text-emerald-400" />
              <span>Pending Verifications ({verifications.filter((v) => v.status === 'pending').length})</span>
            </h3>
          </div>

          <div className="space-y-3">
            {verifications.filter((v) => v.status === 'pending').length === 0 ? (
              <div className="text-xs text-slate-500 py-6 text-center">No pending verification requests in queue.</div>
            ) : (
              verifications
                .filter((v) => v.status === 'pending')
                .map((req) => {
                  const proj = projects.find((p) => p.id === req.projectId);
                  return (
                    <div key={req.id} className="p-3.5 bg-slate-950 border border-slate-800 rounded-xl space-y-2 text-xs">
                      <div className="flex items-center justify-between">
                        <span className="font-bold text-white">{proj?.name || `Project #${req.projectId}`}</span>
                        <span className="font-mono text-purple-400 uppercase font-semibold text-[10px]">{req.badgeLevel}</span>
                      </div>
                      <p className="text-slate-400">{req.notes || 'No notes attached.'}</p>
                      <div className="flex items-center justify-end space-x-2 pt-2 border-t border-slate-900">
                        <button
                          onClick={() => handleRejectVerification(req.id)}
                          className="px-3 py-1 rounded-lg bg-red-500/10 hover:bg-red-500/20 text-red-400 border border-red-500/30 transition flex items-center space-x-1"
                        >
                          <XCircle className="w-3.5 h-3.5" />
                          <span>Reject</span>
                        </button>
                        <button
                          onClick={() => handleApproveVerification(req.id)}
                          className="px-3 py-1 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-semibold transition flex items-center space-x-1"
                        >
                          <CheckCircle2 className="w-3.5 h-3.5" />
                          <span>Approve Badge</span>
                        </button>
                      </div>
                    </div>
                  );
                })
            )}
          </div>
        </div>

        {/* Dispute Moderation Queue */}
        <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 space-y-4">
          <div className="flex items-center justify-between border-b border-slate-800 pb-3">
            <h3 className="font-bold text-white text-base flex items-center space-x-2">
              <AlertTriangle className="w-5 h-5 text-amber-400" />
              <span>Dispute Moderation Queue ({disputes.filter((d) => d.status === 'open').length})</span>
            </h3>
          </div>

          <div className="space-y-3">
            {disputes.filter((d) => d.status === 'open').length === 0 ? (
              <div className="text-xs text-slate-500 py-6 text-center">No open disputes requiring moderation.</div>
            ) : (
              disputes
                .filter((d) => d.status === 'open')
                .map((disp) => {
                  const target = projects.find((p) => p.id === disp.projectId);
                  const dupeOf = projects.find((p) => p.id === disp.duplicateOfProjectId);
                  return (
                    <div key={disp.id} className="p-3.5 bg-slate-950 border border-slate-800 rounded-xl space-y-2 text-xs">
                      <div className="flex items-center justify-between text-slate-200">
                        <span>Reported: <strong className="text-amber-400">{target?.name}</strong></span>
                        <span>Duplicate of: <strong className="text-blue-400">{dupeOf?.name}</strong></span>
                      </div>
                      <p className="text-slate-400 italic">"{disp.reason}"</p>
                      <div className="flex items-center justify-end space-x-2 pt-2 border-t border-slate-900">
                        <button
                          onClick={() => handleResolveDispute(disp.id, 'dismiss')}
                          className="px-3 py-1 rounded-lg bg-slate-800 text-slate-300 hover:bg-slate-700 transition"
                        >
                          Dismiss
                        </button>
                        <button
                          onClick={() => handleResolveDispute(disp.id, 'archive_project')}
                          className="px-3 py-1 rounded-lg bg-red-600 hover:bg-red-500 text-white font-semibold transition"
                        >
                          Archive Duplicate
                        </button>
                      </div>
                    </div>
                  );
                })
            )}
          </div>
        </div>
      </div>

      {/* Fee & Admin Management Section */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Fee Configuration Form */}
        <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 space-y-4">
          <h3 className="font-bold text-white text-base flex items-center space-x-2 border-b border-slate-800 pb-3">
            <Coins className="w-5 h-5 text-amber-400" />
            <span>Protocol Fee & SAC Token Config</span>
          </h3>

          <form onSubmit={handleUpdateFees} className="space-y-4 text-xs">
            <div className="flex items-center justify-between p-3 bg-slate-950 border border-slate-800 rounded-xl">
              <div>
                <div className="font-semibold text-white">Enable Protocol Fees</div>
                <div className="text-slate-400 text-[11px]">Enforce operation fees in XLM or SAC token</div>
              </div>
              <input
                type="checkbox"
                checked={feeEnabled}
                onChange={(e) => setFeeEnabled(e.target.checked)}
                className="w-4 h-4 rounded bg-slate-800 border-slate-700 text-purple-600 focus:ring-0"
              />
            </div>

            <div>
              <label className="block text-slate-300 mb-1 font-semibold">Fee Token / SAC Contract Address</label>
              <input
                type="text"
                value={feeToken}
                onChange={(e) => setFeeToken(e.target.value)}
                className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 font-mono text-slate-300 outline-none"
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-slate-300 mb-1 font-semibold">Registration Fee (XLM)</label>
                <input
                  type="number"
                  value={regFee}
                  onChange={(e) => setRegFee(Number(e.target.value))}
                  className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-white outline-none"
                />
              </div>

              <div>
                <label className="block text-slate-300 mb-1 font-semibold">Verification Fee (XLM)</label>
                <input
                  type="number"
                  value={verFee}
                  onChange={(e) => setVerFee(Number(e.target.value))}
                  className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-white outline-none"
                />
              </div>
            </div>

            <button
              type="submit"
              className="px-4 py-2 rounded-xl bg-purple-600 hover:bg-purple-500 text-white font-semibold transition"
            >
              Update Fee Settings
            </button>
          </form>
        </div>

        {/* Admin Roster & Timelock */}
        <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 space-y-4">
          <h3 className="font-bold text-white text-base flex items-center space-x-2 border-b border-slate-800 pb-3">
            <Key className="w-5 h-5 text-purple-400" />
            <span>Admin Roster & Key Rotation</span>
          </h3>

          <div className="space-y-2">
            <span className="text-[11px] font-mono text-slate-400 uppercase">Current Contract Admins:</span>
            {admins.map((adm, idx) => (
              <div key={idx} className="p-2.5 bg-slate-950 border border-slate-800 rounded-xl flex items-center justify-between text-xs font-mono">
                <span className="text-slate-300">{adm.slice(0, 16)}...{adm.slice(-8)}</span>
                <span className="text-[10px] bg-purple-500/20 text-purple-300 px-2 py-0.5 rounded border border-purple-500/30">SuperAdmin</span>
              </div>
            ))}
          </div>

          <form onSubmit={handleAddAdmin} className="pt-2 flex items-center space-x-2 text-xs">
            <input
              type="text"
              placeholder="G... Stellar Public Key"
              value={newAdminAddress}
              onChange={(e) => setNewAdminAddress(e.target.value)}
              className="flex-1 bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-white font-mono outline-none"
            />
            <button
              type="submit"
              className="px-3 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold transition shrink-0"
            >
              Add Admin
            </button>
          </form>
        </div>
      </div>

      {/* Admin Action Audit Log */}
      <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 space-y-4">
        <h3 className="font-bold text-white text-base">On-Chain Admin Action Audit Log</h3>
        <div className="space-y-2 max-h-60 overflow-y-auto pr-2">
          {adminLogs.map((log) => (
            <div key={log.id} className="p-3 bg-slate-950 border border-slate-800/80 rounded-xl flex items-center justify-between text-xs font-mono">
              <div>
                <span className="text-purple-400 font-bold">{log.action}: </span>
                <span className="text-slate-300 font-sans">{log.details}</span>
              </div>
              <span className="text-[10px] text-slate-500 shrink-0 ml-4">{new Date(log.timestamp).toLocaleTimeString()}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
