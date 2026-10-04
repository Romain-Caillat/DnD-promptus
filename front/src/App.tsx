import { BrowserRouter, Route, Routes } from 'react-router'
import { HealthPage } from '@/features/health/HealthPage'

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="*" element={<HealthPage />} />
      </Routes>
    </BrowserRouter>
  )
}
