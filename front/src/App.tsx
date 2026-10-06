import { BrowserRouter, Route, Routes } from 'react-router'
import { RegisterPage } from '@/features/auth/RegisterPage'
import { SignInPage } from '@/features/auth/SignInPage'
import { CampaignPage } from '@/features/gm/CampaignPage'
import { GmHomePage } from '@/features/gm/GmHomePage'
import { GmTablePage } from '@/features/gm/GmTablePage'
import { NewCampaignPage } from '@/features/gm/NewCampaignPage'
import { PlayerViewPage } from '@/features/gm/PlayerViewPage'
import { HealthPage } from '@/features/health/HealthPage'
import { CharacterCreatorPage } from '@/features/play/creator/CharacterCreatorPage'
import { JoinPage } from '@/features/play/JoinPage'
import { PlayerHomePage } from '@/features/play/PlayerHomePage'
import { ReferencePage } from '@/features/reference/ReferencePage'

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/connexion" element={<SignInPage />} />
        <Route path="/inscription" element={<RegisterPage />} />
        <Route path="/reference" element={<ReferencePage />} />
        <Route path="/sante" element={<HealthPage />} />
        <Route path="/rejoindre/:code" element={<JoinPage />} />
        <Route path="/partie/:campaignId" element={<PlayerHomePage />} />
        <Route path="/partie/:campaignId/personnage" element={<CharacterCreatorPage />} />
        <Route path="/campagnes/nouvelle" element={<NewCampaignPage />} />
        <Route path="/campagnes/:campaignId" element={<CampaignPage />} />
        <Route path="/campagnes/:campaignId/table" element={<GmTablePage />} />
        <Route path="/campagnes/:campaignId/vue-joueurs" element={<PlayerViewPage />} />
        <Route path="*" element={<GmHomePage />} />
      </Routes>
    </BrowserRouter>
  )
}
