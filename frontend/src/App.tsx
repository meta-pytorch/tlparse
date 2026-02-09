import { useManifest } from "./hooks/useManifest";
import { Layout } from "./components/Layout";
import "./styles/globals.css";

export default function App() {
  const { loading, error } = useManifest();

  if (loading) {
    return <div className="loading">Loading tlparse output...</div>;
  }

  if (error) {
    return <div className="error-box">{error}</div>;
  }

  return <Layout />;
}
