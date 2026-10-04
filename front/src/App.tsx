import { BrowserRouter, Route, Routes } from 'react-router'
import { RegisterPage } from '@/features/auth/RegisterPage'
import { SignInPage } from '@/features/auth/SignInPage'
import { GmHomePage } from '@/features/gm/GmHomePage'
import { HealthPage } from '@/features/health/HealthPage'
import { ReferencePage } from '@/features/reference/ReferencePage'

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/connexion" element={<SignInPage />} />
        <Route path="/inscription" element={<RegisterPage />} />
        <Route path="/reference" element={<ReferencePage />} />
        <Route path="/sante" element={<HealthPage />} />
        <Route path="*" element={<GmHomePage />} />
      </Routes>
    </BrowserRouter>
  )
}
