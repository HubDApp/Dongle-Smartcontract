import React, { useState } from 'react';
import { Navbar } from './components/Navbar';
import { ProjectRegistryView } from './components/ProjectRegistryView';
import { CollectionsView } from './components/CollectionsView';
import { AdminConsoleView } from './components/AdminConsoleView';
import { ContractSimulatorView } from './components/ContractSimulatorView';
import { EventStreamView } from './components/EventStreamView';
import { ContractDocsView } from './components/ContractDocsView';
import { ProjectDetailModal } from './components/ProjectDetailModal';
import { RegisterProjectModal } from './components/RegisterProjectModal';
import { SubmitReviewModal } from './components/SubmitReviewModal';
import { contractEngine } from './services/contractEngine';
import { MOCK_USERS } from './data/mockContractData';
import { Project, UserAccount } from './types';

export function App() {
  const [activeTab, setActiveTab] = useState<string>('registry');
  const [currentUser, setCurrentUser] = useState<UserAccount>(MOCK_USERS[0]); // Default to Alice (Admin)
  const [projects, setProjects] = useState<Project[]>(contractEngine.getProjects());

  // Modals state
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [isRegisterModalOpen, setIsRegisterModalOpen] = useState(false);
  const [isReviewModalOpen, setIsReviewModalOpen] = useState(false);
  const [reviewModalProjectId, setReviewModalProjectId] = useState<number | null>(null);

  const refreshState = () => {
    setProjects([...contractEngine.getProjects()]);
    if (selectedProject) {
      const updatedProj = contractEngine.getProjectById(selectedProject.id);
      if (updatedProj) setSelectedProject({ ...updatedProj });
    }
  };

  const handleOpenReviewModal = (projectId: number) => {
    setReviewModalProjectId(projectId);
    setIsReviewModalOpen(true);
  };

  const stats = contractEngine.getStats();

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 flex flex-col font-sans selection:bg-blue-500 selection:text-white">
      {/* Top Navbar */}
      <Navbar
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        currentUser={currentUser}
        setCurrentUser={setCurrentUser}
        stats={stats}
      />

      {/* Main Container */}
      <main className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {activeTab === 'registry' && (
          <ProjectRegistryView
            projects={projects}
            currentUser={currentUser}
            onSelectProject={(p) => setSelectedProject(p)}
            onOpenRegisterModal={() => setIsRegisterModalOpen(true)}
          />
        )}

        {activeTab === 'collections' && (
          <CollectionsView
            currentUser={currentUser}
            onSelectProject={(p) => setSelectedProject(p)}
          />
        )}

        {activeTab === 'admin' && (
          <AdminConsoleView
            currentUser={currentUser}
            onUpdate={refreshState}
          />
        )}

        {activeTab === 'simulator' && (
          <ContractSimulatorView
            currentUser={currentUser}
            onUpdate={refreshState}
          />
        )}

        {activeTab === 'events' && <EventStreamView />}

        {activeTab === 'docs' && <ContractDocsView />}
      </main>

      {/* Footer */}
      <footer className="border-t border-slate-800/80 bg-slate-900/50 py-6 mt-12 text-center text-xs text-slate-500 font-mono">
        <div className="max-w-7xl mx-auto px-4 flex flex-col sm:flex-row items-center justify-between gap-2">
          <div>
            Dongle Smart Contract &copy; {new Date().getFullYear()} Stellar Soroban Protocol
          </div>
          <div>
            Contract WASM: <span className="text-slate-400">CCWUXOTO2RJK...CXJ42N73</span>
          </div>
        </div>
      </footer>

      {/* Modals & Drawers */}
      <ProjectDetailModal
        project={selectedProject}
        onClose={() => setSelectedProject(null)}
        currentUser={currentUser}
        onUpdate={refreshState}
        onOpenReviewModal={handleOpenReviewModal}
      />

      <RegisterProjectModal
        isOpen={isRegisterModalOpen}
        onClose={() => setIsRegisterModalOpen(false)}
        currentUser={currentUser}
        onSuccess={(newProj) => {
          refreshState();
          setSelectedProject(newProj);
        }}
      />

      <SubmitReviewModal
        isOpen={isReviewModalOpen}
        projectId={reviewModalProjectId}
        onClose={() => {
          setIsReviewModalOpen(false);
          setReviewModalProjectId(null);
        }}
        currentUser={currentUser}
        onSuccess={refreshState}
      />
    </div>
  );
}

export default App;
