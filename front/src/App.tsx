import { BrowserRouter, Route, Routes } from 'react-router'
import { HealthPage } from '@/features/health/HealthPage'
import { ReferencePage } from '@/features/reference/ReferencePage'

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/reference" element={<ReferencePage />} />
        <Route path="*" element={<HealthPage />} />
      </Routes>
    </BrowserRouter>
  )
}
